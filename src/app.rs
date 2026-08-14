use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::device::{DeviceStatus, RtlDevice, list_rtl_devices, merge_scan};
use crate::event::AppEvent;
use crate::jobs::{Job, JobState, JobTable};
use crate::tools::catalog::{
    ToolId, default_values, eeprom_backup_path, plan_command, plan_eeprom_write_with_backup,
};
use crate::tools::parse::{parse_lost_bytes, parse_tuner};
use crate::tools::runner::{self, SpawnedJob};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Devices,
    Actions,
    Params,
    Log,
}

impl Focus {
    fn next(self) -> Self {
        match self {
            Self::Devices => Self::Actions,
            Self::Actions => Self::Params,
            Self::Params => Self::Log,
            Self::Log => Self::Devices,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Devices => Self::Log,
            Self::Actions => Self::Devices,
            Self::Params => Self::Actions,
            Self::Log => Self::Params,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Modal {
    Help,
    ConfirmQuit,
    ConfirmEeprom {
        tool: ToolId,
        serial: String,
        index: usize,
        values: Vec<String>,
        backup: PathBuf,
        typed: String,
        summary: String,
    },
}

pub struct App {
    pub devices: Vec<RtlDevice>,
    pub selected: usize,
    pub action: usize,
    pub param_index: usize,
    pub focus: Focus,
    pub values: HashMap<ToolId, Vec<String>>,
    pub editing: bool,
    pub edit_buf: String,
    pub jobs: JobTable,
    pub modal: Option<Modal>,
    pub status: String,
    pub scan_error: Option<String>,
    pub last_scan: Instant,
    pub log_follow: bool,
    pub log_offset: usize,
    pub should_quit: bool,
    tx: Sender<AppEvent>,
}

impl App {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let values = ToolId::ALL
            .iter()
            .map(|id| (*id, default_values(&id.spec())))
            .collect();
        let mut app = Self {
            devices: Vec::new(),
            selected: 0,
            action: 0,
            param_index: 0,
            focus: Focus::Devices,
            values,
            editing: false,
            edit_buf: String::new(),
            jobs: JobTable::default(),
            modal: None,
            status: "scanning USB…".into(),
            scan_error: None,
            last_scan: Instant::now() - Duration::from_secs(10),
            log_follow: true,
            log_offset: 0,
            should_quit: false,
            tx,
        };
        app.rescan();
        app
    }

    pub fn selected_device(&self) -> Option<&RtlDevice> {
        self.devices.get(self.selected)
    }

    pub fn selected_tool(&self) -> ToolId {
        ToolId::ALL[self.action.min(ToolId::ALL.len() - 1)]
    }

    pub fn current_values(&self) -> &[String] {
        self.values
            .get(&self.selected_tool())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn current_job(&self) -> Option<&Job> {
        self.selected_device().and_then(|d| self.jobs.get(&d.id))
    }

    pub fn running_jobs(&self) -> usize {
        self.jobs.running_count()
    }

    pub fn rescan(&mut self) {
        match list_rtl_devices() {
            Ok(fresh) => {
                let selected_id = self.selected_device().map(|d| d.id.clone());
                self.devices = merge_scan(&self.devices, fresh);
                for device in &mut self.devices {
                    if self.jobs.is_running(&device.id) {
                        device.status = DeviceStatus::Running;
                    }
                }
                if let Some(id) = selected_id {
                    if let Some(idx) = self.devices.iter().position(|d| d.id == id) {
                        self.selected = idx;
                    } else if self.selected >= self.devices.len() {
                        self.selected = self.devices.len().saturating_sub(1);
                    }
                } else if !self.devices.is_empty() {
                    // Prefer the lowest serial for first selection.
                    if let Some((idx, _)) = self
                        .devices
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, d)| d.display_serial().to_string())
                    {
                        self.selected = idx;
                    }
                }
                self.scan_error = None;
                self.status = format!("{} RTL-SDR device(s)", self.devices.len());
            }
            Err(e) => {
                self.scan_error = Some(e.clone());
                self.status = e;
            }
        }
        self.last_scan = Instant::now();
    }

    pub fn handle(&mut self, event: AppEvent) {
        match event {
            AppEvent::Tick => self.on_tick(),
            AppEvent::Input(ev) => {
                if let crossterm::event::Event::Key(key) = ev {
                    self.on_key(key);
                }
            }
            AppEvent::JobLine {
                serial,
                stream: _,
                text,
            } => self.on_job_line(serial, text),
        }
    }

    fn on_tick(&mut self) {
        if self.last_scan.elapsed() >= Duration::from_secs(2) {
            self.rescan();
        }
        let done = self.jobs.poll_all();
        for (serial, state) in done {
            if let Some(dev) = self.devices.iter_mut().find(|d| d.id == serial)
                && dev.status == DeviceStatus::Running
            {
                dev.status = DeviceStatus::Idle;
            }
            if let Some(job) = self.jobs.get(&serial) {
                self.status = format!("{} on {} {}", job.tool.spec().name, serial, state.label());
            }
        }
    }

    fn on_job_line(&mut self, serial: String, text: String) {
        if let Some(tuner) = parse_tuner(&text)
            && let Some(dev) = self.devices.iter_mut().find(|d| d.id == serial)
        {
            dev.tuner = Some(tuner);
        }
        if let Some(job) = self.jobs.get_mut(&serial) {
            if let Some(n) = parse_lost_bytes(&text) {
                job.lost_bytes = Some(n);
            }
            job.push_line(text);
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return;
        }
        if let Some(modal) = self.modal.clone() {
            self.on_modal_key(key, modal);
            return;
        }
        if self.editing {
            self.on_edit_key(key);
            return;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.request_quit();
            }
            (KeyCode::Char('?'), _) | (KeyCode::F(1), _) => {
                self.modal = Some(Modal::Help);
            }
            (KeyCode::Tab, KeyModifiers::SHIFT) | (KeyCode::BackTab, _) => {
                self.focus = self.focus.prev();
            }
            (KeyCode::Tab, _) => self.focus = self.focus.next(),
            (KeyCode::Char('r'), _) if self.focus != Focus::Params => self.rescan(),
            (KeyCode::Char('s'), _) => self.stop_selected(),
            (KeyCode::Esc, _) => {
                if self.jobs.is_running(
                    &self
                        .selected_device()
                        .map(|d| d.id.clone())
                        .unwrap_or_default(),
                ) {
                    self.stop_selected();
                }
            }
            _ => match self.focus {
                Focus::Devices => self.on_devices_key(key),
                Focus::Actions => self.on_actions_key(key),
                Focus::Params => self.on_params_key(key),
                Focus::Log => self.on_log_key(key),
            },
        }
    }

    fn on_devices_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_device(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_device(1),
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected = 0;
                self.log_follow = true;
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.selected = self.devices.len().saturating_sub(1);
                self.log_follow = true;
            }
            KeyCode::Enter | KeyCode::Right => self.focus = Focus::Actions,
            _ => {}
        }
    }

    fn on_actions_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.action = self.action.saturating_sub(1);
                self.param_index = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.action + 1 < ToolId::ALL.len() {
                    self.action += 1;
                    self.param_index = 0;
                }
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.action = 0;
                self.param_index = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.action = ToolId::ALL.len() - 1;
                self.param_index = 0;
            }
            KeyCode::Left => self.focus = Focus::Devices,
            KeyCode::Right => {
                self.focus = Focus::Params;
                let n = self.selected_tool().spec().params.len();
                self.param_index = n; // land on Run
            }
            KeyCode::Enter => self.start_selected(),
            _ => {}
        }
    }

    fn on_params_key(&mut self, key: KeyEvent) {
        let spec = self.selected_tool().spec();
        let n = spec.params.len() + 1; // last row is Run
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.param_index = self.param_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.param_index + 1 < n {
                    self.param_index += 1;
                }
            }
            KeyCode::Left => self.focus = Focus::Actions,
            KeyCode::Enter => {
                if self.param_index >= spec.params.len() {
                    self.start_selected();
                } else {
                    self.begin_edit();
                }
            }
            _ => {}
        }
    }

    fn on_log_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.log_follow = false;
                self.log_offset = self.log_offset.saturating_add(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.log_offset = self.log_offset.saturating_sub(1);
                if self.log_offset == 0 {
                    self.log_follow = true;
                }
            }
            KeyCode::PageUp => {
                self.log_follow = false;
                self.log_offset = self.log_offset.saturating_add(10);
            }
            KeyCode::PageDown => {
                self.log_offset = self.log_offset.saturating_sub(10);
                if self.log_offset == 0 {
                    self.log_follow = true;
                }
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.log_follow = false;
                if let Some(job) = self.current_job() {
                    self.log_offset = job.log.len().saturating_sub(1);
                }
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.log_follow = true;
                self.log_offset = 0;
            }
            _ => {}
        }
    }

    fn begin_edit(&mut self) {
        let tool = self.selected_tool();
        let spec = tool.spec();
        if self.param_index >= spec.params.len() {
            return;
        }
        let current = self
            .values
            .get(&tool)
            .and_then(|v| v.get(self.param_index))
            .cloned()
            .unwrap_or_default();
        let param = spec.params[self.param_index];
        if param.kind == crate::tools::catalog::ParamKind::Choice && !param.choices.is_empty() {
            let values = self
                .values
                .entry(tool)
                .or_insert_with(|| default_values(&spec));
            let cur = values[self.param_index].as_str();
            let pos = param.choices.iter().position(|c| *c == cur).unwrap_or(0);
            let next = param.choices[(pos + 1) % param.choices.len()];
            values[self.param_index] = next.to_string();
            return;
        }
        self.edit_buf = current;
        self.editing = true;
    }

    fn on_edit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.editing = false;
                self.edit_buf.clear();
            }
            KeyCode::Enter => {
                let tool = self.selected_tool();
                let spec = tool.spec();
                if self.param_index < spec.params.len() {
                    let values = self
                        .values
                        .entry(tool)
                        .or_insert_with(|| default_values(&spec));
                    values[self.param_index] = self.edit_buf.clone();
                }
                self.editing = false;
                self.edit_buf.clear();
            }
            KeyCode::Backspace => {
                self.edit_buf.pop();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.edit_buf.push(c);
            }
            _ => {}
        }
    }

    fn on_modal_key(&mut self, key: KeyEvent, modal: Modal) {
        match modal {
            Modal::Help => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Enter
                ) {
                    self.modal = None;
                }
            }
            Modal::ConfirmQuit => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.stop_all();
                    self.should_quit = true;
                }
                KeyCode::Char('n') | KeyCode::Esc => self.modal = None,
                _ => {}
            },
            Modal::ConfirmEeprom {
                tool,
                serial,
                index,
                values,
                backup,
                mut typed,
                summary,
            } => match key.code {
                KeyCode::Esc => self.modal = None,
                KeyCode::Backspace => {
                    typed.pop();
                    self.modal = Some(Modal::ConfirmEeprom {
                        tool,
                        serial,
                        index,
                        values,
                        backup,
                        typed,
                        summary,
                    });
                }
                KeyCode::Enter => {
                    if typed == "WRITE" {
                        self.modal = None;
                        self.start_eeprom_write(tool, &serial, index, &values, &backup);
                    }
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if typed.len() < 16 {
                        typed.push(c);
                    }
                    self.modal = Some(Modal::ConfirmEeprom {
                        tool,
                        serial,
                        index,
                        values,
                        backup,
                        typed,
                        summary,
                    });
                }
                _ => {}
            },
        }
    }

    fn move_device(&mut self, delta: i32) {
        if self.devices.is_empty() {
            return;
        }
        let n = self.devices.len() as i32;
        let next = (self.selected as i32 + delta).clamp(0, n - 1) as usize;
        if next != self.selected {
            self.selected = next;
            self.log_follow = true;
            self.log_offset = 0;
        }
    }

    fn request_quit(&mut self) {
        if self.jobs.running_count() > 0 {
            self.modal = Some(Modal::ConfirmQuit);
        } else {
            self.should_quit = true;
        }
    }

    fn stop_selected(&mut self) {
        let Some(id) = self.selected_device().map(|d| d.id.clone()) else {
            return;
        };
        self.stop_serial(&id);
    }

    fn stop_serial(&mut self, serial: &str) {
        if let Some(job) = self.jobs.get_mut(serial)
            && job.state == JobState::Running
        {
            job.stop_requested = true;
            runner::stop_group(job.pid);
            job.push_line("[rtlutil] stop requested (SIGTERM)".into());
            self.status = format!("stopping job on {serial}");
        }
    }

    fn stop_all(&mut self) {
        let serials: Vec<String> = self
            .jobs
            .iter()
            .filter(|j| j.state == JobState::Running)
            .map(|j| j.serial.clone())
            .collect();
        for s in serials {
            self.stop_serial(&s);
        }
    }

    fn start_selected(&mut self) {
        let Some(dev) = self.selected_device().cloned() else {
            self.status = "no device selected".into();
            return;
        };
        if self.jobs.is_running(&dev.id) {
            self.status = format!("{} already has a running job — press s to stop", dev.id);
            return;
        }
        // Refresh indices immediately before index-based tools.
        self.rescan();
        let Some(dev) = self.devices.iter().find(|d| d.id == dev.id).cloned() else {
            self.status = "device disappeared during rescan".into();
            return;
        };

        let tool = self.selected_tool();
        let spec = tool.spec();
        let values = self
            .values
            .get(&tool)
            .cloned()
            .unwrap_or_else(|| default_values(&spec));

        if spec.dangerous {
            let backup = eeprom_backup_path(dev.display_serial());
            let summary = eeprom_summary(tool, &values);
            self.modal = Some(Modal::ConfirmEeprom {
                tool,
                serial: dev.id.clone(),
                index: dev.index,
                values,
                backup,
                typed: String::new(),
                summary,
            });
            return;
        }

        match plan_command(&spec, dev.display_serial(), dev.index, &values) {
            Ok(planned) => self.spawn_job(&dev, tool, planned),
            Err(e) => self.status = e,
        }
    }

    fn start_eeprom_write(
        &mut self,
        tool: ToolId,
        serial: &str,
        index: usize,
        values: &[String],
        backup: &Path,
    ) {
        let spec = tool.spec();
        match plan_eeprom_write_with_backup(&spec, serial, index, values, backup) {
            Ok(planned) => {
                if let Some(dev) = self.devices.iter().find(|d| d.id == serial).cloned() {
                    self.spawn_job(&dev, tool, planned);
                } else {
                    self.status = "device gone before write".into();
                }
            }
            Err(e) => self.status = e,
        }
    }

    fn spawn_job(
        &mut self,
        dev: &RtlDevice,
        tool: ToolId,
        planned: crate::tools::catalog::PlannedCommand,
    ) {
        let display = planned.display.clone();
        match runner::spawn(&dev.id, planned, self.tx.clone()) {
            Ok(SpawnedJob { child, pid }) => {
                let mut job = Job {
                    serial: dev.id.clone(),
                    tool,
                    display: display.clone(),
                    started: Instant::now(),
                    child: Some(child),
                    pid,
                    state: JobState::Running,
                    log: Default::default(),
                    lost_bytes: None,
                    stop_requested: false,
                };
                job.push_line(format!("$ {}", display.join(" ")));
                self.jobs.insert(job);
                if let Some(d) = self.devices.iter_mut().find(|d| d.id == dev.id) {
                    d.status = DeviceStatus::Running;
                }
                self.log_follow = true;
                self.log_offset = 0;
                self.focus = Focus::Log;
                self.status = format!("{} started on {}", tool.spec().name, dev.display_serial());
            }
            Err(e) => {
                self.status = e;
            }
        }
    }
}

fn eeprom_summary(tool: ToolId, values: &[String]) -> String {
    let spec = tool.spec();
    spec.params
        .iter()
        .zip(values.iter())
        .map(|(p, v)| format!("{} = {v}", p.label))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::catalog::DeviceArg;

    #[test]
    fn serial_tools_never_use_index() {
        for id in ToolId::ALL {
            if matches!(
                id,
                ToolId::EepromRead
                    | ToolId::EepromDump
                    | ToolId::EepromWrite
                    | ToolId::EepromPreset
                    | ToolId::BiasT
            ) {
                assert_eq!(id.spec().device_arg, DeviceArg::Index, "{id:?}");
            } else {
                assert_eq!(id.spec().device_arg, DeviceArg::Serial, "{id:?}");
            }
        }
    }
}

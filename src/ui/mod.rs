use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::{App, Focus, Modal};
use crate::jobs::JobState;
use crate::tools::catalog::ToolId;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let [title, body, output, status] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(3),
        Constraint::Fill(2),
        Constraint::Length(1),
    ])
    .areas(area);

    draw_title(frame, title, app);

    let [devs, detail] =
        Layout::horizontal([Constraint::Length(26), Constraint::Fill(1)]).areas(body);
    draw_devices(frame, devs, app);
    draw_detail(frame, detail, app);
    draw_output(frame, output, app);
    draw_status(frame, status, app);

    match &app.modal {
        Some(Modal::Help) => draw_help(frame, area),
        Some(Modal::ConfirmQuit) => draw_quit(frame, area, app.running_jobs()),
        Some(Modal::ConfirmEeprom {
            serial,
            backup,
            typed,
            summary,
            ..
        }) => draw_eeprom_confirm(
            frame,
            area,
            serial,
            backup.to_string_lossy().as_ref(),
            typed,
            summary,
        ),
        None => {}
    }
}

fn draw_title(frame: &mut Frame, area: Rect, app: &App) {
    let running = app.running_jobs();
    let jobs = if running == 0 {
        "no jobs".to_string()
    } else {
        format!("{running} running")
    };
    let line = Line::from(vec![
        Span::styled(
            " rtlutil ",
            Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  {} RTL-SDR   {jobs}", app.devices.len())),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn pane_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let style = if focused {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new().fg(Color::DarkGray)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(title)
}

fn draw_devices(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Devices;
    let block = pane_block("Devices", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.devices.is_empty() {
        let msg = app
            .scan_error
            .as_deref()
            .unwrap_or("No RTL-SDR devices found.\nConnect a dongle or press r.");
        frame.render_widget(
            Paragraph::new(msg).style(Style::new().fg(Color::Yellow)),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = app
        .devices
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let marker = if i == app.selected { "▸" } else { " " };
            let status_style = match d.status {
                crate::device::DeviceStatus::Running => Style::new().fg(Color::Green),
                crate::device::DeviceStatus::Busy => Style::new().fg(Color::Yellow),
                crate::device::DeviceStatus::Gone => Style::new().fg(Color::Red),
                crate::device::DeviceStatus::Idle => Style::new().fg(Color::Gray),
            };
            let sel = if i == app.selected {
                Style::new().add_modifier(Modifier::BOLD)
            } else {
                Style::new()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{marker} "), sel),
                Span::styled(format!("{:<10}", d.display_serial()), sel.fg(Color::White)),
                Span::styled(format!(" {}", d.status.label()), status_style),
            ]))
        })
        .collect();
    frame.render_widget(List::new(items), inner);
}

fn draw_detail(frame: &mut Frame, area: Rect, app: &App) {
    let [header, rest] = Layout::vertical([Constraint::Length(5), Constraint::Fill(1)]).areas(area);
    draw_header(frame, header, app);

    let [actions, params] =
        Layout::vertical([Constraint::Length(15), Constraint::Fill(1)]).areas(rest);
    draw_actions(frame, actions, app);
    draw_params(frame, params, app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let block = pane_block("Device", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(dev) = app.selected_device() else {
        frame.render_widget(Paragraph::new("No device selected"), inner);
        return;
    };
    let tuner = dev.tuner.as_deref().unwrap_or("unknown");
    let mfg = dev.manufacturer.as_deref().unwrap_or("—");
    let product = dev.product.as_deref().unwrap_or("—");
    let text = vec![
        Line::from(format!(
            "SN {}   idx {}   {tuner}",
            dev.display_serial(),
            dev.index
        )),
        Line::from(dev.usb_label()),
        Line::from(format!("{mfg}  {product}")),
    ];
    frame.render_widget(Paragraph::new(text), inner);
}

fn draw_actions(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Actions;
    let block = pane_block("Actions", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = ToolId::ALL
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let spec = id.spec();
            let marker = if i == app.action { "▸" } else { " " };
            let danger = if spec.dangerous { " !" } else { "" };
            let style = if i == app.action {
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if spec.dangerous {
                Style::new().fg(Color::Yellow)
            } else {
                Style::new()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{marker} {:<14}", spec.name), style),
                Span::styled(
                    format!(" {:<10}{danger}", spec.bin),
                    Style::new().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();
    frame.render_widget(List::new(items), inner);
}

fn draw_params(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Params;
    let spec = app.selected_tool().spec();
    let block = pane_block("Parameters  (enter edit / run)", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let values = app.current_values();
    let mut lines: Vec<Line> = Vec::new();
    for (i, param) in spec.params.iter().enumerate() {
        let marker = if focused && i == app.param_index {
            "▸"
        } else {
            " "
        };
        let raw = values.get(i).map(|s| s.as_str()).unwrap_or(param.default);
        let shown = if app.editing && focused && i == app.param_index {
            format!("{}▌", app.edit_buf)
        } else {
            raw.to_string()
        };
        let style = if focused && i == app.param_index {
            Style::new().fg(Color::Cyan)
        } else {
            Style::new()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{marker} {:<22} ", param.label), style),
            Span::styled(shown, style.add_modifier(Modifier::BOLD)),
        ]));
    }
    let run_idx = spec.params.len();
    let run_focus = focused && app.param_index == run_idx;
    let run_style = if run_focus {
        Style::new()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(Color::Green)
    };
    let marker = if run_focus { "▸" } else { " " };
    lines.push(Line::from(Span::styled(
        format!("{marker} [ Run {} ]", spec.name),
        run_style,
    )));
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_output(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Log;
    let title = match app.current_job() {
        Some(job) => {
            let elapsed = job.elapsed();
            let mm = elapsed.as_secs() / 60;
            let ss = elapsed.as_secs() % 60;
            let lost = job
                .lost_bytes
                .map(|n| format!("   lost {n} B"))
                .unwrap_or_default();
            format!(
                "Output  ·  {}  ·  {}  ·  {mm:02}:{ss:02}  ·  {}{lost}",
                job.tool.spec().name,
                job.serial,
                job.state.label()
            )
        }
        None => "Output".to_string(),
    };
    let block = pane_block(&title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(job) = app.current_job() else {
        frame.render_widget(
            Paragraph::new("Select a device and run an action. Output appears here.")
                .style(Style::new().fg(Color::DarkGray)),
            inner,
        );
        return;
    };

    let height = inner.height as usize;
    let total = job.log.len();
    let end = if app.log_follow {
        total
    } else {
        total.saturating_sub(app.log_offset)
    };
    let start = end.saturating_sub(height);
    let lines: Vec<Line> = job
        .log
        .iter()
        .skip(start)
        .take(height)
        .map(|l| {
            let style = if l.text.contains("lost at least") && !l.text.contains("lost at least 0") {
                Style::new().fg(Color::Yellow)
            } else if l.text.starts_with('$') || l.text.starts_with("[rtlutil]") {
                Style::new().fg(Color::Cyan)
            } else if l.text.to_ascii_lowercase().contains("error") || l.text.contains("Failed") {
                Style::new().fg(Color::Red)
            } else {
                Style::new()
            };
            Line::from(Span::styled(l.text.clone(), style))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let keys = " ↑↓ device   tab pane   enter run/edit   s stop   r refresh   ? help   q quit ";
    let msg = format!(" {keys}  │  {} ", app.status);
    frame.render_widget(
        Paragraph::new(msg).style(Style::new().bg(Color::DarkGray).fg(Color::White)),
        area,
    );
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let popup = centered(area, 64, 20);
    frame.render_widget(Clear, popup);
    let text = vec![
        Line::from(Span::styled(
            "rtlutil keys",
            Style::new().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  tab / shift-tab     cycle Devices → Actions → Params → Log"),
        Line::from("  ↑ ↓ j k             move in the focused pane"),
        Line::from("  enter               run the action (or edit a parameter)"),
        Line::from("  →                   open parameters (cursor on Run)"),
        Line::from("  choices             enter cycles the value"),
        Line::from("  s / esc             stop the selected device's job"),
        Line::from("  r                   rescan USB"),
        Line::from("  q                   quit (confirms if jobs are running)"),
        Line::from(""),
        Line::from("Devices are addressed by serial, never by USB index."),
        Line::from("EEPROM write dumps a backup, then requires typing WRITE."),
        Line::from(""),
        Line::from("  esc / ? / enter     close this help"),
    ];
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Help")
                    .border_style(Style::new().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn draw_quit(frame: &mut Frame, area: Rect, running: usize) {
    let popup = centered(area, 52, 7);
    frame.render_widget(Clear, popup);
    let text = vec![
        Line::from(format!("{running} job(s) still running.")),
        Line::from("Stop them and quit?"),
        Line::from(""),
        Line::from("  y / enter   yes      n / esc   no"),
    ];
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Quit")
                .border_style(Style::new().fg(Color::Yellow)),
        ),
        popup,
    );
}

fn draw_eeprom_confirm(
    frame: &mut Frame,
    area: Rect,
    serial: &str,
    backup: &str,
    typed: &str,
    summary: &str,
) {
    let popup = centered(area, 70, 16);
    frame.render_widget(Clear, popup);
    let mut lines = vec![
        Line::from(Span::styled(
            "EEPROM WRITE  —  this programs the dongle",
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("Device SN {serial}")),
        Line::from(format!("Backup    {backup}")),
        Line::from(""),
    ];
    for row in summary.lines() {
        lines.push(Line::from(format!("  {row}")));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Type WRITE and press enter to program.  esc cancels.",
    ));
    lines.push(Line::from(format!("> {typed}")));
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Confirm EEPROM write")
                    .border_style(Style::new().fg(Color::Red)),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

/// Silence unused import if JobState is only used later.
#[allow(dead_code)]
fn _job_state_style(state: JobState) -> Style {
    match state {
        JobState::Running => Style::new().fg(Color::Green),
        JobState::Exited(0) => Style::new().fg(Color::Gray),
        JobState::Exited(_) | JobState::Failed => Style::new().fg(Color::Red),
        JobState::Stopped => Style::new().fg(Color::Yellow),
    }
}

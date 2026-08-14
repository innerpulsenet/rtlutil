use std::collections::{HashMap, VecDeque};
use std::process::Child;
use std::time::Instant;

use crate::tools::catalog::ToolId;

pub const LOG_CAP: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Running,
    Exited(i32),
    Stopped,
    Failed,
}

impl JobState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Exited(_) => "exited",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub text: String,
}

pub struct Job {
    pub serial: String,
    pub tool: ToolId,
    pub display: Vec<String>,
    pub started: Instant,
    pub child: Option<Child>,
    pub pid: u32,
    pub state: JobState,
    pub log: VecDeque<LogLine>,
    pub lost_bytes: Option<u64>,
    pub stop_requested: bool,
}

impl Job {
    pub fn push_line(&mut self, text: String) {
        if self.log.len() >= LOG_CAP {
            self.log.pop_front();
        }
        self.log.push_back(LogLine { text });
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.started.elapsed()
    }

    pub fn poll_exit(&mut self) -> bool {
        let Some(child) = self.child.as_mut() else {
            return false;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                if self.stop_requested {
                    self.state = JobState::Stopped;
                } else if status.success() {
                    self.state = JobState::Exited(status.code().unwrap_or(0));
                } else {
                    self.state = JobState::Exited(status.code().unwrap_or(1));
                }
                self.child = None;
                true
            }
            Ok(None) => false,
            Err(e) => {
                self.push_line(format!("wait failed: {e}"));
                self.state = JobState::Failed;
                self.child = None;
                true
            }
        }
    }
}

#[derive(Default)]
pub struct JobTable {
    by_serial: HashMap<String, Job>,
}

impl JobTable {
    pub fn get(&self, serial: &str) -> Option<&Job> {
        self.by_serial.get(serial)
    }

    pub fn get_mut(&mut self, serial: &str) -> Option<&mut Job> {
        self.by_serial.get_mut(serial)
    }

    pub fn insert(&mut self, job: Job) {
        self.by_serial.insert(job.serial.clone(), job);
    }

    pub fn running_count(&self) -> usize {
        self.by_serial
            .values()
            .filter(|j| j.state == JobState::Running)
            .count()
    }

    pub fn is_running(&self, serial: &str) -> bool {
        self.by_serial
            .get(serial)
            .is_some_and(|j| j.state == JobState::Running)
    }

    pub fn poll_all(&mut self) -> Vec<(String, JobState)> {
        let mut done = Vec::new();
        for job in self.by_serial.values_mut() {
            if job.state == JobState::Running && job.poll_exit() {
                done.push((job.serial.clone(), job.state));
            }
        }
        done
    }

    pub fn iter(&self) -> impl Iterator<Item = &Job> {
        self.by_serial.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_caps() {
        let mut job = Job {
            serial: "1".into(),
            tool: ToolId::Test,
            display: vec![],
            started: Instant::now(),
            child: None,
            pid: 0,
            state: JobState::Running,
            log: VecDeque::new(),
            lost_bytes: None,
            stop_requested: false,
        };
        for i in 0..(LOG_CAP + 10) {
            job.push_line(format!("line {i}"));
        }
        assert_eq!(job.log.len(), LOG_CAP);
        assert_eq!(job.log.front().unwrap().text, format!("line {}", 10));
    }
}

//! Spawn rtl_* processes, stream output, and stop the process group.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

use super::catalog::{PlannedCommand, StdoutPolicy};
use crate::event::{AppEvent, LineStream};

pub struct SpawnedJob {
    pub child: Child,
    pub pid: u32,
}

pub fn spawn(
    serial: &str,
    planned: PlannedCommand,
    tx: Sender<AppEvent>,
) -> Result<SpawnedJob, String> {
    let stdout = match &planned.stdout {
        StdoutPolicy::Log => Stdio::piped(),
        StdoutPolicy::File(path) => {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
            }
            let file = std::fs::File::create(path)
                .map_err(|e| format!("cannot create {}: {e}", path.display()))?;
            Stdio::from(file)
        }
        StdoutPolicy::Discard => Stdio::null(),
    };

    let mut cmd = Command::new(&planned.program);
    cmd.args(&planned.args)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start {}: {e}", planned.program.display()))?;
    let pid = child.id();

    if matches!(planned.stdout, StdoutPolicy::Log)
        && let Some(out) = child.stdout.take()
    {
        pump(out, tx.clone(), serial.to_string(), LineStream::Stdout);
    }
    if let Some(err) = child.stderr.take() {
        pump(err, tx.clone(), serial.to_string(), LineStream::Stderr);
    }

    let wait_serial = serial.to_string();
    let wait_tx = tx;
    // Take ownership of Child for the waiter by re-spawning wait on a duplicate...
    // We keep Child in the caller and poll try_wait from the UI thread instead.
    let _ = (wait_serial, wait_tx, pid);

    Ok(SpawnedJob { child, pid })
}

fn pump(
    reader: impl Read + Send + 'static,
    tx: Sender<AppEvent>,
    serial: String,
    stream: LineStream,
) {
    thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        let mut acc = Vec::new();
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    emit_acc(&tx, &serial, stream, &mut acc);
                    break;
                }
                Ok(n) => {
                    for &b in &buf[..n] {
                        if b == b'\n' || b == b'\r' {
                            emit_acc(&tx, &serial, stream, &mut acc);
                        } else if acc.len() < 8192 {
                            acc.push(b);
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
}

fn emit_acc(tx: &Sender<AppEvent>, serial: &str, stream: LineStream, acc: &mut Vec<u8>) {
    if acc.is_empty() {
        return;
    }
    let text = String::from_utf8_lossy(acc).into_owned();
    acc.clear();
    let _ = tx.send(AppEvent::JobLine {
        serial: serial.to_string(),
        stream,
        text,
    });
}

pub fn stop_group(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGTERM);
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
    }
}

pub fn force_stop_group(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
    }
}

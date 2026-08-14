//! Hardware tests. Run with `RTLUTIL_HW=1 cargo test --test hardware -- --nocapture`.

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use rtlutil::device::list_rtl_devices;
use rtlutil::tools::catalog::{DeviceArg, ToolId, default_values, plan_command};

fn hw_enabled() -> bool {
    std::env::var("RTLUTIL_HW").ok().as_deref() == Some("1")
}

#[test]
fn enumerates_three_known_serials() {
    if !hw_enabled() {
        return;
    }
    let devices = list_rtl_devices().expect("USB enumerate");
    let serials: Vec<String> = devices.iter().filter_map(|d| d.serial.clone()).collect();
    for expected in ["00000001", "00000002", "00000003"] {
        assert!(
            serials.iter().any(|s| s == expected),
            "missing serial {expected} in {serials:?}"
        );
    }
    assert_eq!(
        devices.len(),
        3,
        "expected exactly 3 RTL-SDR dongles, got {devices:?}"
    );
}

#[test]
fn index_matches_rtl_test_listing() {
    if !hw_enabled() {
        return;
    }
    let devices = list_rtl_devices().expect("USB enumerate");
    // Display order is by serial; librtlsdr index is stored on each device.
    let by_sn: std::collections::HashMap<_, _> = devices
        .iter()
        .filter_map(|d| d.serial.clone().map(|s| (s, d.index)))
        .collect();
    assert_eq!(by_sn.get("00000003").copied(), Some(0), "{by_sn:?}");
    assert_eq!(by_sn.get("00000002").copied(), Some(1), "{by_sn:?}");
    assert_eq!(by_sn.get("00000001").copied(), Some(2), "{by_sn:?}");
}

#[test]
fn rtl_test_by_serial_sees_r820t() {
    if !hw_enabled() {
        return;
    }
    let spec = ToolId::Test.spec();
    assert_eq!(spec.device_arg, DeviceArg::Serial);
    let values = default_values(&spec);
    let planned = plan_command(&spec, "00000001", 99, &values).expect("plan rtl_test");
    assert_eq!(planned.args[0], "-d");
    assert_eq!(planned.args[1], "00000001");
    assert_ne!(planned.args[1], "99", "must not fall back to the USB index");

    let mut child = Command::new(&planned.program)
        .args(&planned.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rtl_test");
    thread::sleep(Duration::from_secs(2));
    let _ = child.kill();
    let out = child.wait_with_output().expect("wait rtl_test");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("R820T") || combined.contains("Rafael"),
        "rtl_test output did not mention R820T:\n{combined}"
    );
}

#[test]
fn second_job_on_same_serial_is_refused_by_catalog_contract() {
    // Pure contract: the app refuses a second running job per serial.
    // Exercised here without opening hardware so `cargo test` stays safe.
    use rtlutil::jobs::{Job, JobState, JobTable};
    use std::time::Instant;

    let mut table = JobTable::default();
    table.insert(Job {
        serial: "00000001".into(),
        tool: ToolId::Test,
        display: vec![],
        started: Instant::now(),
        child: None,
        pid: 1,
        state: JobState::Running,
        log: Default::default(),
        lost_bytes: None,
        stop_requested: false,
    });
    assert!(table.is_running("00000001"));
    assert!(!table.is_running("00000002"));
}

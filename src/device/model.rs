//! RTL-SDR device identity and status.

use std::fmt;

pub const RTL_VID: u16 = 0x0bda;
pub const RTL_PIDS: [u16; 2] = [0x2832, 0x2838];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    Idle,
    Running,
    Busy,
    Gone,
}

impl DeviceStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "run",
            Self::Busy => "busy",
            Self::Gone => "gone",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RtlDevice {
    /// Stable id: USB serial, or `bus:addr` if the serial is empty.
    pub id: String,
    pub serial: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub vid: u16,
    pub pid: u16,
    pub bus: u8,
    pub address: u8,
    /// librtlsdr index (USB bus+address order). Used only for tools
    /// whose `-d` flag is an index, not a serial (`rtl_eeprom`, `rtl_biast`).
    pub index: usize,
    pub tuner: Option<String>,
    pub status: DeviceStatus,
}

impl RtlDevice {
    pub fn display_serial(&self) -> &str {
        self.serial.as_deref().unwrap_or(self.id.as_str())
    }

    pub fn usb_label(&self) -> String {
        format!(
            "{:04x}:{:04x}  bus {} dev {}",
            self.vid, self.pid, self.bus, self.address
        )
    }
}

impl fmt::Display for RtlDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}  {}", self.display_serial(), self.status.label())
    }
}

pub fn is_rtl_sdr(vid: u16, pid: u16) -> bool {
    vid == RTL_VID && RTL_PIDS.contains(&pid)
}

/// Stable device id: prefer the USB serial string.
pub fn device_id(serial: Option<&str>, bus: u8, address: u8) -> String {
    match serial {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => format!("{bus}:{address}"),
    }
}

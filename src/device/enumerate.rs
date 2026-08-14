//! Native USB enumeration of RTL-SDR dongles via nusb.

use nusb::MaybeFuture;

use super::model::{DeviceStatus, RtlDevice, device_id, is_rtl_sdr};

/// List connected RTL2832U devices, ordered the way librtlsdr walks USB
/// (bus number, then device address). That order is the `-d` index for
/// tools that do not accept a serial.
pub fn list_rtl_devices() -> Result<Vec<RtlDevice>, String> {
    let infos = nusb::list_devices()
        .wait()
        .map_err(|e| format!("USB enumeration failed: {e}"))?;

    let mut devices: Vec<RtlDevice> = infos
        .filter(|info| is_rtl_sdr(info.vendor_id(), info.product_id()))
        .map(|info| {
            let bus = info.busnum();
            let address = info.device_address();
            let serial = info
                .serial_number()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            RtlDevice {
                id: device_id(serial.as_deref(), bus, address),
                serial,
                manufacturer: info.manufacturer_string().map(str::to_string),
                product: info.product_string().map(str::to_string),
                vid: info.vendor_id(),
                pid: info.product_id(),
                bus,
                address,
                index: 0,
                tuner: None,
                status: DeviceStatus::Idle,
            }
        })
        .collect();

    // librtlsdr/libusb on Linux lists these dongles high address first
    // (rtl_test idx 0 is SN 00000003 on this hub). Assign that index,
    // then sort the list by serial so the TUI reads 1, 2, 3.
    devices.sort_by_key(|d| std::cmp::Reverse((d.bus, d.address)));
    for (index, device) in devices.iter_mut().enumerate() {
        device.index = index;
    }
    devices.sort_by(|a, b| a.display_serial().cmp(b.display_serial()));
    Ok(devices)
}

/// Merge a fresh USB scan into the existing list, preserving tuner cache
/// and keeping the same selected id when possible.
pub fn merge_scan(old: &[RtlDevice], fresh: Vec<RtlDevice>) -> Vec<RtlDevice> {
    fresh
        .into_iter()
        .map(|mut d| {
            if let Some(prev) = old.iter().find(|p| p.id == d.id) {
                if d.tuner.is_none() {
                    d.tuner = prev.tuner.clone();
                }
                if prev.status == DeviceStatus::Running {
                    d.status = DeviceStatus::Running;
                }
            }
            d
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(id: &str, bus: u8, addr: u8, index: usize) -> RtlDevice {
        RtlDevice {
            id: id.to_string(),
            serial: Some(id.to_string()),
            manufacturer: None,
            product: None,
            vid: 0x0bda,
            pid: 0x2838,
            bus,
            address: addr,
            index,
            tuner: None,
            status: DeviceStatus::Idle,
        }
    }

    #[test]
    fn merge_preserves_tuner_and_running() {
        let mut old = vec![dev("00000001", 2, 6, 2)];
        old[0].tuner = Some("R820T".into());
        old[0].status = DeviceStatus::Running;
        let fresh = vec![dev("00000001", 2, 6, 0)];
        let merged = merge_scan(&old, fresh);
        assert_eq!(merged[0].tuner.as_deref(), Some("R820T"));
        assert_eq!(merged[0].status, DeviceStatus::Running);
        assert_eq!(merged[0].index, 0);
    }

    #[test]
    fn merge_drops_unplugged() {
        let old = vec![dev("00000001", 2, 6, 0), dev("00000002", 2, 5, 1)];
        let fresh = vec![dev("00000002", 2, 5, 0)];
        let merged = merge_scan(&old, fresh);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, "00000002");
    }
}

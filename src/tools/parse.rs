//! Light parsers for rtl_* text output.

/// Pull a tuner name out of an osmocom banner line.
pub fn parse_tuner(line: &str) -> Option<String> {
    let line = line.trim();
    let rest = line.strip_prefix("Found ")?.strip_suffix(" tuner")?;
    let name = rest.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Parse `rtl_test` / librtlsdr listing lines:
/// `  0:  Realtek, RTL2838UHIDIR, SN: 00000003`
pub fn parse_device_listing(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some((idx_s, rest)) = line.split_once(':') else {
            continue;
        };
        let Ok(idx) = idx_s.trim().parse::<usize>() else {
            continue;
        };
        let Some(sn) = rest.split("SN:").nth(1) else {
            continue;
        };
        let serial = sn.trim().to_string();
        if !serial.is_empty() {
            out.push((idx, serial));
        }
    }
    out
}

/// Last "lost at least N bytes" count, if this line reports one.
pub fn parse_lost_bytes(line: &str) -> Option<u64> {
    let line = line.trim();
    let rest = line.strip_prefix("lost at least ")?;
    let num = rest.strip_suffix(" bytes")?;
    num.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuner_r820t() {
        assert_eq!(
            parse_tuner("Found Rafael Micro R820T tuner"),
            Some("Rafael Micro R820T".into())
        );
    }

    #[test]
    fn tuner_ignores_other_lines() {
        assert_eq!(parse_tuner("Sampling at 2048000 S/s."), None);
        assert_eq!(parse_tuner("Found 3 device(s):"), None);
    }

    #[test]
    fn listing_maps_serials() {
        let text = "\
Found 3 device(s):
  0:  Realtek, RTL2838UHIDIR, SN: 00000003
  1:  Realtek, RTL2838UHIDIR, SN: 00000002
  2:  Realtek, RTL2838UHIDIR, SN: 00000001

Using device 0: Generic RTL2832U OEM
";
        assert_eq!(
            parse_device_listing(text),
            vec![
                (0, "00000003".into()),
                (1, "00000002".into()),
                (2, "00000001".into()),
            ]
        );
    }

    #[test]
    fn lost_bytes() {
        assert_eq!(parse_lost_bytes("lost at least 12 bytes"), Some(12));
        assert_eq!(parse_lost_bytes("lost at least 0 bytes"), Some(0));
        assert_eq!(parse_lost_bytes("hello"), None);
    }
}

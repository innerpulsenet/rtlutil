//! Data-driven catalog of rtl_* actions and argv builders.

use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    Test,
    Ppm,
    TunerBench,
    EepromRead,
    EepromDump,
    EepromWrite,
    EepromPreset,
    BiasT,
    Power,
    Record,
    Fm,
    Tcp,
    Adsb,
}

impl ToolId {
    pub const ALL: [ToolId; 13] = [
        Self::Test,
        Self::Ppm,
        Self::TunerBench,
        Self::EepromRead,
        Self::EepromDump,
        Self::EepromWrite,
        Self::EepromPreset,
        Self::BiasT,
        Self::Power,
        Self::Record,
        Self::Fm,
        Self::Tcp,
        Self::Adsb,
    ];
}

impl fmt::Display for ToolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.spec().name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceArg {
    /// `-d <serial>` — rtl_test, rtl_fm, rtl_sdr, rtl_power, rtl_tcp, rtl_adsb
    Serial,
    /// `-d <index>` — rtl_eeprom, rtl_biast (serial is parsed as an integer)
    Index,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    Text,
    File,
    Audio,
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    Text,
    Int,
    Choice,
    Path,
}

#[derive(Debug, Clone, Copy)]
pub struct Param {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: ParamKind,
    pub default: &'static str,
    pub choices: &'static [&'static str],
    /// CLI flag, or `None` for a positional argument.
    pub flag: Option<&'static str>,
    pub omit_if_empty: bool,
    /// Flag-only: include the flag with no value when the field is `true`/`1`/`yes`.
    pub flag_only: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ToolSpec {
    pub id: ToolId,
    pub name: &'static str,
    pub bin: &'static str,
    pub summary: &'static str,
    pub device_arg: DeviceArg,
    pub output: OutputKind,
    pub dangerous: bool,
    pub params: &'static [Param],
}

impl ToolId {
    pub fn spec(self) -> ToolSpec {
        match self {
            Self::Test => TEST,
            Self::Ppm => PPM,
            Self::TunerBench => TUNER_BENCH,
            Self::EepromRead => EEPROM_READ,
            Self::EepromDump => EEPROM_DUMP,
            Self::EepromWrite => EEPROM_WRITE,
            Self::EepromPreset => EEPROM_PRESET,
            Self::BiasT => BIAST,
            Self::Power => POWER,
            Self::Record => RECORD,
            Self::Fm => FM,
            Self::Tcp => TCP,
            Self::Adsb => ADSB,
        }
    }
}

const SAMPLE_RATE: Param = Param {
    key: "sample_rate",
    label: "sample rate (Hz)",
    kind: ParamKind::Int,
    default: "2048000",
    choices: &[],
    flag: Some("-s"),
    omit_if_empty: true,
    flag_only: false,
};

const GAIN: Param = Param {
    key: "gain",
    label: "gain (0 = auto)",
    kind: ParamKind::Text,
    default: "0",
    choices: &[],
    flag: Some("-g"),
    omit_if_empty: true,
    flag_only: false,
};

const PPM_CORR: Param = Param {
    key: "ppm",
    label: "PPM correction",
    kind: ParamKind::Int,
    default: "0",
    choices: &[],
    flag: Some("-p"),
    omit_if_empty: true,
    flag_only: false,
};

const FREQ: Param = Param {
    key: "freq",
    label: "frequency (Hz)",
    kind: ParamKind::Text,
    default: "100000000",
    choices: &[],
    flag: Some("-f"),
    omit_if_empty: false,
    flag_only: false,
};

const BIAS_T: Param = Param {
    key: "bias_t",
    label: "bias-T",
    kind: ParamKind::Choice,
    default: "off",
    choices: &["off", "on"],
    flag: Some("-T"),
    omit_if_empty: true,
    flag_only: true,
};

const TEST: ToolSpec = ToolSpec {
    id: ToolId::Test,
    name: "Test",
    bin: "rtl_test",
    summary: "Lost-sample / throughput bench (runs until stopped)",
    device_arg: DeviceArg::Serial,
    output: OutputKind::Text,
    dangerous: false,
    params: &[SAMPLE_RATE],
};

const PPM: ToolSpec = ToolSpec {
    id: ToolId::Ppm,
    name: "PPM measure",
    bin: "rtl_test",
    summary: "Estimate crystal PPM error",
    device_arg: DeviceArg::Serial,
    output: OutputKind::Text,
    dangerous: false,
    params: &[Param {
        key: "seconds",
        label: "seconds",
        kind: ParamKind::Int,
        default: "10",
        choices: &[],
        flag: Some("-p"),
        omit_if_empty: false,
        flag_only: false,
    }],
};

const TUNER_BENCH: ToolSpec = ToolSpec {
    id: ToolId::TunerBench,
    name: "Tuner bench",
    bin: "rtl_test",
    summary: "E4000 tuner range test (R820T will abort)",
    device_arg: DeviceArg::Serial,
    output: OutputKind::Text,
    dangerous: false,
    params: &[SAMPLE_RATE],
};

const EEPROM_READ: ToolSpec = ToolSpec {
    id: ToolId::EepromRead,
    name: "EEPROM read",
    bin: "rtl_eeprom",
    summary: "Print current EEPROM strings (no write)",
    device_arg: DeviceArg::Index,
    output: OutputKind::Text,
    dangerous: false,
    params: &[],
};

const EEPROM_DUMP: ToolSpec = ToolSpec {
    id: ToolId::EepromDump,
    name: "EEPROM dump",
    bin: "rtl_eeprom",
    summary: "Write a raw EEPROM image to a file",
    device_arg: DeviceArg::Index,
    output: OutputKind::Text,
    dangerous: false,
    params: &[Param {
        key: "file",
        label: "dump file",
        kind: ParamKind::Path,
        default: "eeprom.bin",
        choices: &[],
        flag: Some("-r"),
        omit_if_empty: false,
        flag_only: false,
    }],
};

const EEPROM_WRITE: ToolSpec = ToolSpec {
    id: ToolId::EepromWrite,
    name: "EEPROM write",
    bin: "rtl_eeprom",
    summary: "Program manufacturer / product / serial (typed confirm)",
    device_arg: DeviceArg::Index,
    output: OutputKind::Text,
    dangerous: true,
    params: &[
        Param {
            key: "manufacturer",
            label: "manufacturer",
            kind: ParamKind::Text,
            default: "Realtek",
            choices: &[],
            flag: Some("-m"),
            omit_if_empty: true,
            flag_only: false,
        },
        Param {
            key: "product",
            label: "product",
            kind: ParamKind::Text,
            default: "RTL2838UHIDIR",
            choices: &[],
            flag: Some("-p"),
            omit_if_empty: true,
            flag_only: false,
        },
        Param {
            key: "serial",
            label: "serial",
            kind: ParamKind::Text,
            default: "",
            choices: &[],
            flag: Some("-s"),
            omit_if_empty: false,
            flag_only: false,
        },
        Param {
            key: "ir",
            label: "IR endpoint",
            kind: ParamKind::Choice,
            default: "1",
            choices: &["0", "1"],
            flag: Some("-i"),
            omit_if_empty: true,
            flag_only: false,
        },
    ],
};

const EEPROM_PRESET: ToolSpec = ToolSpec {
    id: ToolId::EepromPreset,
    name: "EEPROM preset",
    bin: "rtl_eeprom",
    summary: "Write a factory EEPROM profile (typed confirm)",
    device_arg: DeviceArg::Index,
    output: OutputKind::Text,
    dangerous: true,
    params: &[Param {
        key: "preset",
        label: "preset",
        kind: ParamKind::Choice,
        default: "realtek_oem",
        choices: &[
            "realtek",
            "realtek_oem",
            "noxon",
            "terratec_black",
            "terratec_plus",
        ],
        flag: Some("-g"),
        omit_if_empty: false,
        flag_only: false,
    }],
};

const BIAST: ToolSpec = ToolSpec {
    id: ToolId::BiasT,
    name: "Bias-T",
    bin: "rtl_biast",
    summary: "Enable or disable bias-T / GPIO",
    device_arg: DeviceArg::Index,
    output: OutputKind::Text,
    dangerous: false,
    params: &[
        Param {
            key: "bias",
            label: "bias-T",
            kind: ParamKind::Choice,
            default: "1",
            choices: &["0", "1"],
            flag: Some("-b"),
            omit_if_empty: false,
            flag_only: false,
        },
        Param {
            key: "gpio",
            label: "GPIO pin",
            kind: ParamKind::Int,
            default: "0",
            choices: &[],
            flag: Some("-g"),
            omit_if_empty: true,
            flag_only: false,
        },
    ],
};

const POWER: ToolSpec = ToolSpec {
    id: ToolId::Power,
    name: "Power scan",
    bin: "rtl_power",
    summary: "FFT logger over a frequency range",
    device_arg: DeviceArg::Serial,
    output: OutputKind::Text,
    dangerous: false,
    params: &[
        Param {
            key: "range",
            label: "range (low:high:bin)",
            kind: ParamKind::Text,
            default: "88M:108M:125k",
            choices: &[],
            flag: Some("-f"),
            omit_if_empty: false,
            flag_only: false,
        },
        Param {
            key: "interval",
            label: "integration interval",
            kind: ParamKind::Text,
            default: "10",
            choices: &[],
            flag: Some("-i"),
            omit_if_empty: true,
            flag_only: false,
        },
        Param {
            key: "single",
            label: "single shot",
            kind: ParamKind::Choice,
            default: "yes",
            choices: &["yes", "no"],
            flag: Some("-1"),
            omit_if_empty: true,
            flag_only: true,
        },
        GAIN,
        PPM_CORR,
        Param {
            key: "file",
            label: "CSV file (optional)",
            kind: ParamKind::Path,
            default: "",
            choices: &[],
            flag: None,
            omit_if_empty: true,
            flag_only: false,
        },
    ],
};

const RECORD: ToolSpec = ToolSpec {
    id: ToolId::Record,
    name: "I/Q record",
    bin: "rtl_sdr",
    summary: "Capture raw I/Q samples to a file",
    device_arg: DeviceArg::Serial,
    output: OutputKind::File,
    dangerous: false,
    params: &[
        FREQ,
        SAMPLE_RATE,
        GAIN,
        PPM_CORR,
        Param {
            key: "samples",
            label: "sample count (0 = until stop)",
            kind: ParamKind::Int,
            default: "0",
            choices: &[],
            flag: Some("-n"),
            omit_if_empty: true,
            flag_only: false,
        },
        Param {
            key: "file",
            label: "output file",
            kind: ParamKind::Path,
            default: "capture.iq",
            choices: &[],
            flag: None,
            omit_if_empty: false,
            flag_only: false,
        },
    ],
};

const FM: ToolSpec = ToolSpec {
    id: ToolId::Fm,
    name: "FM demod",
    bin: "rtl_fm",
    summary: "Narrowband demod to a file, or pipe to aplay",
    device_arg: DeviceArg::Serial,
    output: OutputKind::Audio,
    dangerous: false,
    params: &[
        FREQ,
        Param {
            key: "mod",
            label: "modulation",
            kind: ParamKind::Choice,
            default: "fm",
            choices: &["fm", "wbfm", "am", "usb", "lsb", "raw"],
            flag: Some("-M"),
            omit_if_empty: true,
            flag_only: false,
        },
        Param {
            key: "sample_rate",
            label: "sample rate (Hz)",
            kind: ParamKind::Int,
            default: "24000",
            choices: &[],
            flag: Some("-s"),
            omit_if_empty: true,
            flag_only: false,
        },
        GAIN,
        PPM_CORR,
        BIAS_T,
        Param {
            key: "output",
            label: "output (file path or aplay)",
            kind: ParamKind::Text,
            default: "aplay",
            choices: &[],
            flag: None,
            omit_if_empty: false,
            flag_only: false,
        },
    ],
};

const TCP: ToolSpec = ToolSpec {
    id: ToolId::Tcp,
    name: "TCP server",
    bin: "rtl_tcp",
    summary: "I/Q server for remote clients (runs until stopped)",
    device_arg: DeviceArg::Serial,
    output: OutputKind::Network,
    dangerous: false,
    params: &[
        Param {
            key: "addr",
            label: "listen address",
            kind: ParamKind::Text,
            default: "127.0.0.1",
            choices: &[],
            flag: Some("-a"),
            omit_if_empty: true,
            flag_only: false,
        },
        Param {
            key: "port",
            label: "listen port",
            kind: ParamKind::Int,
            default: "1234",
            choices: &[],
            flag: Some("-p"),
            omit_if_empty: true,
            flag_only: false,
        },
        FREQ,
        SAMPLE_RATE,
        GAIN,
        Param {
            key: "ppm",
            label: "PPM correction",
            kind: ParamKind::Int,
            default: "0",
            choices: &[],
            flag: Some("-P"),
            omit_if_empty: true,
            flag_only: false,
        },
        BIAS_T,
    ],
};

const ADSB: ToolSpec = ToolSpec {
    id: ToolId::Adsb,
    name: "ADS-B",
    bin: "rtl_adsb",
    summary: "Decode Mode-S / ADS-B frames to the log",
    device_arg: DeviceArg::Serial,
    output: OutputKind::Text,
    dangerous: false,
    params: &[GAIN, PPM_CORR, BIAS_T],
};

#[derive(Debug, Clone)]
pub struct PlannedCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    /// Shown in the log header (program + args).
    pub display: Vec<String>,
    pub stdout: StdoutPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdoutPolicy {
    Log,
    File(PathBuf),
    /// stdout is binary or handed to another process; only stderr is logged.
    Discard,
}

pub fn default_values(spec: &ToolSpec) -> Vec<String> {
    spec.params.iter().map(|p| p.default.to_string()).collect()
}

pub fn param_map<'a>(spec: &ToolSpec, values: &'a [String]) -> Vec<(&'static str, &'a str)> {
    spec.params
        .iter()
        .zip(values.iter())
        .map(|(p, v)| (p.key, v.as_str()))
        .collect()
}

pub fn get_param<'a>(spec: &ToolSpec, values: &'a [String], key: &str) -> Option<&'a str> {
    spec.params
        .iter()
        .zip(values.iter())
        .find(|(p, _)| p.key == key)
        .map(|(_, v)| v.as_str())
}

fn flag_only_on(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn validate_value(param: &Param, value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() {
        if param.omit_if_empty {
            return Ok(());
        }
        return Err(format!("{} is required", param.label));
    }
    match param.kind {
        ParamKind::Int => {
            if value.parse::<i64>().is_err() {
                return Err(format!("{} must be an integer", param.label));
            }
        }
        ParamKind::Choice if !param.choices.is_empty() => {
            if !param.choices.contains(&value) {
                return Err(format!(
                    "{} must be one of: {}",
                    param.label,
                    param.choices.join(", ")
                ));
            }
        }
        ParamKind::Path | ParamKind::Text | ParamKind::Choice => {}
    }
    Ok(())
}

pub fn validate(spec: &ToolSpec, values: &[String]) -> Result<(), String> {
    if values.len() != spec.params.len() {
        return Err("internal: param count mismatch".into());
    }
    for (param, value) in spec.params.iter().zip(values.iter()) {
        validate_value(param, value)?;
    }
    if spec.id == ToolId::EepromWrite {
        let serial = get_param(spec, values, "serial").unwrap_or("").trim();
        if serial.is_empty() {
            return Err("serial is required for EEPROM write".into());
        }
    }
    Ok(())
}

fn which(bin: &str) -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(bin);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    let fallback = PathBuf::from("/usr/bin").join(bin);
    if fallback.is_file() {
        return Ok(fallback);
    }
    Err(format!("`{bin}` not found in PATH or /usr/bin"))
}

fn device_flag(spec: &ToolSpec, serial: &str, index: usize) -> String {
    match spec.device_arg {
        DeviceArg::Serial => serial.to_string(),
        DeviceArg::Index => index.to_string(),
    }
}

fn push_params(spec: &ToolSpec, values: &[String], args: &mut Vec<String>) -> Result<(), String> {
    for (param, value) in spec.params.iter().zip(values.iter()) {
        let value = value.trim();
        if param.flag_only {
            if flag_only_on(value)
                && let Some(flag) = param.flag
            {
                args.push(flag.to_string());
            }
            continue;
        }
        if value.is_empty() {
            if param.omit_if_empty {
                continue;
            }
            return Err(format!("{} is required", param.label));
        }
        // rtl_sdr -n 0 means infinite; skip the flag.
        if param.key == "samples" && value == "0" {
            continue;
        }
        if let Some(flag) = param.flag {
            args.push(flag.to_string());
            args.push(value.to_string());
        } else {
            args.push(value.to_string());
        }
    }
    Ok(())
}

/// Build the process to spawn. `rtl_fm` with output `aplay` becomes a shell pipeline.
pub fn plan_command(
    spec: &ToolSpec,
    serial: &str,
    index: usize,
    values: &[String],
) -> Result<PlannedCommand, String> {
    validate(spec, values)?;
    let program = which(spec.bin)?;
    let mut args = vec!["-d".to_string(), device_flag(spec, serial, index)];

    if spec.id == ToolId::TunerBench {
        args.push("-t".to_string());
    }

    match spec.id {
        ToolId::Fm => return plan_fm(program, args, spec, values),
        ToolId::Record => return plan_record(program, args, spec, values),
        _ => push_params(spec, values, &mut args)?,
    }

    let display = display_argv(&program, &args);
    Ok(PlannedCommand {
        program,
        args,
        display,
        stdout: StdoutPolicy::Log,
    })
}

fn plan_record(
    program: PathBuf,
    mut args: Vec<String>,
    spec: &ToolSpec,
    values: &[String],
) -> Result<PlannedCommand, String> {
    push_params(spec, values, &mut args)?;
    let file = get_param(spec, values, "file")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "output file is required".to_string())?;
    let display = display_argv(&program, &args);
    Ok(PlannedCommand {
        program,
        args,
        display,
        stdout: StdoutPolicy::File(PathBuf::from(file)),
    })
}

fn plan_fm(
    program: PathBuf,
    mut args: Vec<String>,
    spec: &ToolSpec,
    values: &[String],
) -> Result<PlannedCommand, String> {
    let output = get_param(spec, values, "output")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "output is required".to_string())?;

    // Build rtl_fm args without the positional output.
    for (param, value) in spec.params.iter().zip(values.iter()) {
        if param.key == "output" {
            continue;
        }
        let value = value.trim();
        if param.flag_only {
            if flag_only_on(value)
                && let Some(flag) = param.flag
            {
                args.push(flag.to_string());
            }
            continue;
        }
        if value.is_empty() {
            if param.omit_if_empty {
                continue;
            }
            return Err(format!("{} is required", param.label));
        }
        if let Some(flag) = param.flag {
            args.push(flag.to_string());
            args.push(value.to_string());
        }
    }

    if output.eq_ignore_ascii_case("aplay") {
        let rate = get_param(spec, values, "sample_rate").unwrap_or("24000");
        let sh = which("sh")?;
        let rtl = program.to_string_lossy();
        let quoted_rtl = shell_single(&rtl);
        let quoted_args: Vec<String> = args.iter().map(|a| shell_single(a)).collect();
        let script = format!(
            "{quoted_rtl} {} | aplay -t raw -r {rate} -f S16_LE -c 1",
            quoted_args.join(" ")
        );
        let sh_args = vec!["-c".to_string(), script.clone()];
        return Ok(PlannedCommand {
            program: sh,
            args: sh_args,
            display: vec!["sh".into(), "-c".into(), script],
            stdout: StdoutPolicy::Discard,
        });
    }

    args.push(output.to_string());
    let display = display_argv(&program, &args);
    Ok(PlannedCommand {
        program,
        args,
        display,
        stdout: StdoutPolicy::Discard,
    })
}

fn display_argv(program: &Path, args: &[String]) -> Vec<String> {
    let mut out = vec![
        program
            .file_name()
            .unwrap_or(program.as_os_str())
            .to_string_lossy()
            .into_owned(),
    ];
    out.extend(args.iter().cloned());
    out
}

fn shell_single(s: &str) -> String {
    if s.is_empty() {
        return "''".into();
    }
    if s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/' | b':'))
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', r"'\''"))
}

pub fn eeprom_backup_path(old_serial: &str) -> PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_else(|| "/tmp".into());
    let dir = PathBuf::from(home).join(".local/share/rtlutil");
    let ts = chrono_like_stamp();
    dir.join(format!("eeprom-backup-{old_serial}-{ts}.bin"))
}

fn chrono_like_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

/// Dump-then-write as a single `sh -c` job so a failed backup never writes.
pub fn plan_eeprom_write_with_backup(
    spec: &ToolSpec,
    serial: &str,
    index: usize,
    values: &[String],
    backup: &Path,
) -> Result<PlannedCommand, String> {
    validate(spec, values)?;
    let eeprom = which("rtl_eeprom")?;
    let sh = which("sh")?;
    if let Some(parent) = backup.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create backup dir {}: {e}", parent.display()))?;
    }
    let eeprom_s = shell_single(&eeprom.to_string_lossy());
    let backup_s = shell_single(&backup.to_string_lossy());
    let mut write_args = vec!["-d".to_string(), index.to_string()];
    push_params(spec, values, &mut write_args)?;
    let write_s: Vec<String> = write_args.iter().map(|a| shell_single(a)).collect();
    let script = format!(
        "{eeprom_s} -d {index} -r {backup_s} && {eeprom_s} {}",
        write_s.join(" ")
    );
    Ok(PlannedCommand {
        program: sh,
        args: vec!["-c".into(), script.clone()],
        display: vec![
            "rtl_eeprom".into(),
            format!("backup+write SN {serial} idx {index}"),
        ],
        stdout: StdoutPolicy::Log,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uses_serial_not_index() {
        let spec = ToolId::Test.spec();
        assert_eq!(spec.device_arg, DeviceArg::Serial);
        let values = default_values(&spec);
        let mut args = vec!["-d".into(), device_flag(&spec, "00000001", 2)];
        push_params(&spec, &values, &mut args).unwrap();
        assert_eq!(args, ["-d", "00000001", "-s", "2048000"]);
    }

    #[test]
    fn eeprom_uses_index() {
        let spec = ToolId::EepromRead.spec();
        assert_eq!(spec.device_arg, DeviceArg::Index);
        assert_eq!(device_flag(&spec, "00000001", 2), "2");
    }

    #[test]
    fn eeprom_write_requires_serial() {
        let spec = ToolId::EepromWrite.spec();
        let mut values = default_values(&spec);
        let idx = spec.params.iter().position(|p| p.key == "serial").unwrap();
        values[idx] = String::new();
        assert!(validate(&spec, &values).is_err());
        values[idx] = "00000004".into();
        assert!(validate(&spec, &values).is_ok());
    }

    #[test]
    fn tcp_port_is_p_ppm_is_capital_p() {
        let spec = ToolId::Tcp.spec();
        let mut values = default_values(&spec);
        let port_i = spec.params.iter().position(|p| p.key == "port").unwrap();
        let ppm_i = spec.params.iter().position(|p| p.key == "ppm").unwrap();
        values[port_i] = "2345".into();
        values[ppm_i] = "15".into();
        let mut args = vec!["-d".into(), "00000001".into()];
        push_params(&spec, &values, &mut args).unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("-p 2345"), "{joined}");
        assert!(joined.contains("-P 15"), "{joined}");
        assert!(joined.contains("-d 00000001"), "{joined}");
    }

    #[test]
    fn ppm_measure_uses_minus_p_seconds() {
        let spec = ToolId::Ppm.spec();
        let values = default_values(&spec);
        let mut args = vec!["-d".into(), "00000002".into()];
        push_params(&spec, &values, &mut args).unwrap();
        assert_eq!(args, ["-d", "00000002", "-p", "10"]);
    }

    #[test]
    fn flag_only_bias_t() {
        let spec = ToolId::Adsb.spec();
        let mut values = default_values(&spec);
        let i = spec.params.iter().position(|p| p.key == "bias_t").unwrap();
        values[i] = "on".into();
        let mut args = Vec::new();
        push_params(&spec, &values, &mut args).unwrap();
        assert!(args.contains(&"-T".to_string()));
        values[i] = "off".into();
        args.clear();
        push_params(&spec, &values, &mut args).unwrap();
        assert!(!args.contains(&"-T".to_string()));
    }

    #[test]
    fn record_skips_zero_sample_count() {
        let spec = ToolId::Record.spec();
        let values = default_values(&spec);
        let mut args = vec!["-d".into(), "00000001".into()];
        push_params(&spec, &values, &mut args).unwrap();
        assert!(!args.contains(&"-n".to_string()));
        assert!(args.iter().any(|a| a == "capture.iq"));
    }

    #[test]
    fn invalid_choice_rejected() {
        let spec = ToolId::BiasT.spec();
        let mut values = default_values(&spec);
        values[0] = "maybe".into();
        assert!(validate(&spec, &values).is_err());
    }

    #[test]
    fn shell_single_quotes_spaces() {
        assert_eq!(shell_single("foo"), "foo");
        assert_eq!(shell_single("a b"), "'a b'");
        assert_eq!(shell_single("it's"), r"'it'\''s'");
    }
}

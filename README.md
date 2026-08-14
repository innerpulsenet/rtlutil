# rtlutil

A keyboard-first terminal UI for RTL-SDR USB dongles. It lists every Realtek RTL2832U device on the bus, lets you pick one, and runs the usual Osmocom `rtl_*` tools against it with live output.

Devices are addressed **by serial number**, not USB index. On a hub those two orders are often different.

## Requirements

- Linux (x86_64)
- The [rtl-sdr](https://osmocom.org/projects/rtl-sdr/wiki) tools on your `PATH`: `rtl_test`, `rtl_eeprom`, `rtl_biast`, `rtl_power`, `rtl_sdr`, `rtl_fm`, `rtl_tcp`, `rtl_adsb`
- Permission to open the USB devices (the same access `rtl_test` needs)

Debian / Ubuntu:

```bash
sudo apt install rtl-sdr
```

## Install

Download the Linux binary from the [latest release](https://github.com/innerpulsenet/rtlutil/releases/latest), then:

```bash
chmod +x rtlutil-*-x86_64-unknown-linux-gnu
sudo mv rtlutil-*-x86_64-unknown-linux-gnu /usr/local/bin/rtlutil
rtlutil
```

`rtlutil` is only the TUI. The `rtl-sdr` package above still has to be installed.

Or, if you have a Rust toolchain:

```bash
cargo install --git https://github.com/innerpulsenet/rtlutil --locked
```

## Usage

```bash
rtlutil          # start the TUI
rtlutil --list   # print connected dongles and exit
```

| Key | Action |
|---|---|
| `tab` / `shift-tab` | Cycle Devices → Actions → Params → Log |
| `↑` `↓` `j` `k` | Move in the focused pane |
| `enter` | Run the highlighted action, or edit a parameter |
| `→` | Open the parameter form (cursor lands on **Run**) |
| `s` / `esc` | Stop the selected device's job |
| `r` | Rescan USB |
| `?` | Help |
| `q` | Quit (asks first if jobs are running) |

Choice fields cycle on `enter`. Text and number fields edit in place: `enter` commits, `esc` cancels.

Jobs on different dongles can run at the same time. A device that already has a job must be stopped before starting another.

## Actions

| Action | Tool | Notes |
|---|---|---|
| Test | `rtl_test` | Lost-sample bench until you stop it |
| PPM measure | `rtl_test -p` | Crystal error estimate |
| Tuner bench | `rtl_test -t` | E4000 only; R820T will abort |
| EEPROM read / dump | `rtl_eeprom` | Read-only |
| EEPROM write / preset | `rtl_eeprom` | **Dangerous** — see below |
| Bias-T | `rtl_biast` | GPIO 0 by default |
| Power scan | `rtl_power` | Frequency range FFT log |
| I/Q record | `rtl_sdr` | Writes a file, not the log |
| FM demod | `rtl_fm` | File, or `aplay` for speakers |
| TCP server | `rtl_tcp` | I/Q server until stopped |
| ADS-B | `rtl_adsb` | Decoded frames in the log |

The device list is sorted by serial. The `idx` in the detail pane is the librtlsdr / `rtl_test` index, which may not match serial order. `rtl_eeprom` and `rtl_biast` only accept an index; rtlutil maps the selected serial to the current index before launching those tools. Everything else is started with `-d <serial>`.

## EEPROM writes

Programming the EEPROM can make a dongle unusable if the image is wrong.

1. Fill in manufacturer / product / serial, or pick a preset.
2. Press Run. A modal shows the new values and the backup path.
3. Type `WRITE` and press enter.
4. rtlutil writes a backup to `~/.local/share/rtlutil/eeprom-backup-<serial>-<unix>.bin` first, then programs the dongle. If the dump fails, the write does not run.

## Build from source

```bash
git clone https://github.com/innerpulsenet/rtlutil.git
cd rtlutil
cargo build --release
./target/release/rtlutil
```

You need Rust 1.85 or newer (edition 2024).

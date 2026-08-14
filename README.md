# rtlutil

Keyboard-first TUI for enumerating, testing, and configuring RTL-SDR USB dongles.

It lists every Realtek RTL2832U device on the bus, lets you pick one, and runs the Osmocom `rtl_*` tools against it with live output. Devices are always addressed **by serial number**, not USB index — on a hub the index order is not the serial order.

## Requirements

- Linux
- Rust 1.85+ (edition 2024)
- `rtl-sdr` tools on `PATH` (`rtl_test`, `rtl_eeprom`, `rtl_biast`, `rtl_power`, `rtl_sdr`, `rtl_fm`, `rtl_tcp`, `rtl_adsb`)
- Permission to open the USB devices (the same access `rtl_test` needs)

## Build and run

```bash
cargo build --release
./target/release/rtlutil
./target/release/rtlutil --list
```

## Keys

| Key | Action |
|---|---|
| `tab` / `shift-tab` | Cycle Devices → Actions → Params → Log |
| `↑` `↓` `j` `k` | Move in the focused pane |
| `enter` | Run the highlighted action (or edit a parameter) |
| `→` | Open the parameter form (lands on **Run**) |
| `s` / `esc` | Stop the selected device's job |
| `r` | Rescan USB |
| `?` | Help |
| `q` | Quit (asks first if jobs are running) |

Choice fields cycle on `enter`. Text/int fields edit in place; `enter` commits, `esc` cancels.

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

The device list is sorted by serial (`00000001`, `00000002`, …). The `idx` shown in the detail pane is the librtlsdr / `rtl_test` index, which on a hub is often **not** the same order as the serials (on this host, SN `00000003` is index 0).

`rtl_eeprom` and `rtl_biast` only accept a device **index**. rtlutil rescans USB and maps the selected serial to the current index immediately before spawning those tools. Everything else gets `-d <serial>`.

## EEPROM writes

Programming the EEPROM can make a dongle unusable if the image is wrong.

1. Fill in manufacturer / product / serial (or pick a preset).
2. Press Run. A modal shows the new values and the backup path.
3. Type `WRITE` and press enter.
4. rtlutil dumps `~/.local/share/rtlutil/eeprom-backup-<serial>-<unix>.bin` first, then programs. If the dump fails, the write does not run.

Automated tests never write the EEPROM.

## Tests

```bash
cargo test
RTLUTIL_HW=1 cargo test --test hardware -- --nocapture
```

Hardware tests expect three dongles with serials `00000001`, `00000002`, and `00000003`.

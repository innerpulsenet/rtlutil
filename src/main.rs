fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--list") | Some("-l") => {
            list_devices();
            Ok(())
        }
        Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some(other) => {
            eprintln!("unknown argument: {other}");
            print_help();
            std::process::exit(2);
        }
        None => rtlutil::run(),
    }
}

fn print_help() {
    eprintln!(
        "rtlutil — TUI for RTL-SDR devices\n\n  rtlutil          start the TUI\n  rtlutil --list   print connected RTL-SDR devices\n  rtlutil --help   this help"
    );
}

fn list_devices() {
    match rtlutil::device::list_rtl_devices() {
        Ok(devices) if devices.is_empty() => {
            println!("No RTL-SDR devices found.");
        }
        Ok(devices) => {
            println!("Found {} RTL-SDR device(s):", devices.len());
            for d in devices {
                let serial = d.display_serial();
                let mfg = d.manufacturer.as_deref().unwrap_or("?");
                let product = d.product.as_deref().unwrap_or("?");
                println!(
                    "  idx {:>2}  SN {:<12}  {:04x}:{:04x}  bus {} dev {:<3}  {mfg} {product}",
                    d.index, serial, d.vid, d.pid, d.bus, d.address
                );
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

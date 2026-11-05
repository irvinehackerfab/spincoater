//! This crate provides a TUI for the PC connecting to the spincoater's ESP32.

use std::io;

use clap::Parser;
use color_eyre::{Result, eyre::Context};
use host_tui::{
    Args,
    app::{App, SEND_BUFFER, SERIAL_PORT, event::READ_BUFFER},
};
use sc_messages::icd::BAUD_RATE;
use serial2::{CharSize, Parity, SerialPort, Settings, StopBits};
use serialport::{SerialPortType, available_ports};
use std::io::Write;

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();
    let path = if let Some(path) = args.serial_port {
        path
    } else {
        let ports = available_ports()
            .wrap_err("Failed to query available ports")?
            .into_iter()
            .filter(|port| !matches!(port.port_type, SerialPortType::Unknown));
        let stdout = io::stdout();
        {
            let mut out = stdout.lock();
            writeln!(out, "Detected serial port(s) on:")?;
            for port in ports {
                writeln!(out, "{port:#?}")?;
            }
            write!(out, "Please choose a `port_name` to connect to: ")?;
            out.flush()?;
        }
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        // Remove newline
        buffer.truncate(buffer.trim_end().len());
        buffer.into()
    };
    let serial = SerialPort::open(&path, |mut settings: Settings| {
        settings.set_raw();
        settings.set_baud_rate(BAUD_RATE)?;
        settings.set_parity(Parity::None);
        settings.set_char_size(CharSize::Bits8);
        settings.set_stop_bits(StopBits::One);
        Ok(settings)
    })
    .wrap_err_with(|| format!("Failed to open {}", path.display()))?;
    let serial = SERIAL_PORT.init(serial);

    let terminal = ratatui::init();
    let result = App::new(serial, SEND_BUFFER.take(), READ_BUFFER.take())?.run(terminal);
    ratatui::restore();
    result
}

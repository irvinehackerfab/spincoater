//! This crate provides a TUI for the PC connecting to the spincoater's ESP32.

use std::io;

use color_eyre::{
    Result,
    eyre::{Context, eyre},
};
use host_tui::app::{App, SEND_BUFFER, SERIAL_PORT, event::READ_BUFFER};
use sc_messages::icd::BAUD_RATE;
use serial2::{CharSize, Parity, SerialPort, Settings, StopBits};
use serialport::{SerialPortType, available_ports};
use std::io::Write;

fn main() -> Result<()> {
    color_eyre::install()?;

    let ports = available_ports()
        .wrap_err("Failed to query available ports")?
        .into_iter()
        .filter(|port| !matches!(port.port_type, SerialPortType::Unknown))
        .collect::<Vec<_>>();
    if ports.is_empty() {
        return Err(eyre!(
            "No serial ports available. Please plug one in and run this program again."
        ));
    }
    let stdout = io::stdout();
    {
        let mut out = stdout.lock();
        writeln!(out, "Detected serial port(s) on: {ports:#?}")?;
        write!(out, "Please choose a `port_name` connect to: ")?;
        out.flush()?;
    }
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    let serial = SerialPort::open(buffer.trim(), |mut settings: Settings| {
        settings.set_raw();
        settings.set_baud_rate(BAUD_RATE)?;
        settings.set_parity(Parity::None);
        settings.set_char_size(CharSize::Bits8);
        settings.set_stop_bits(StopBits::One);
        Ok(settings)
    })
    .wrap_err("Failed to open serial port")?;
    let serial = SERIAL_PORT.init(serial);

    let terminal = ratatui::init();
    let result = App::new(serial, SEND_BUFFER.take(), READ_BUFFER.take())?.run(terminal);
    ratatui::restore();
    result
}

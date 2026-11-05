//! This crate contains functionality used by the host terminal user interface.

use clap::Parser;
use std::path::PathBuf;

pub mod app;

/// The Vendor ID that shows up when you connect an ESP32 `DevKitC` to a PC over USB.
pub const DEV_KIT_C_VENDOR_ID: u16 = 4292;

/// The Vendor ID that shows up when you connect an ESP-Prog-2 to a PC over USB.
pub const ESP_PROG_2_VENDOR_ID: u16 = 12346;

/// Communicates with the spincoater over a serial port
#[derive(Debug, Parser)]
#[command(about)]
pub struct Args {
    /// The path to the serial port.
    ///
    /// If you leave this blank, the program will list the options and let you choose.
    #[arg(short, long)]
    pub serial_port: Option<PathBuf>,
}

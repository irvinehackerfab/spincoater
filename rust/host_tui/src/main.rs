//! This crate provides a TUI for the PC connecting to the spincoater's ESP32.

use std::io;

use color_eyre::{Result, eyre::eyre};
<<<<<<< HEAD
use postcard_rpc::{header::VarSeqKind, host_client::HostClient};
use sc_messages::icd::BAUD_RATE;
use tokio_serial::available_ports;
=======
use host_tui::{
    DEV_KIT_C_VENDOR_ID, ESP_PROG_2_VENDOR_ID, TX_QUEUE_SIZE, VAR_SEQUENCE_KIND, app::App,
};
use postcard_rpc::{host_client::HostClient, standard_icd::ERROR_PATH};
use sc_messages::icd::BAUD_RATE;
use std::io::Write;
use tokio_serial::{SerialPortType, available_ports};
>>>>>>> main

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

<<<<<<< HEAD
/// The URI that the MCU can use to report "unrecognized request" errors.
const ERROR_PATH: &str = "error";

/// The size of the outgoing queue.
const TX_QUEUE_SIZE: usize = 128;

/// The size of sequuence numbers used when making requests.
///
/// [`postcard_rpc`] gives no hint as to what this should be.
const VAR_SEQUENCE_KIND: VarSeqKind = VarSeqKind::Seq2;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let ports = available_ports()?;
    println!("Available ports: {ports:#?}");
    println!("Please choose a \"port_name\" to connect to: ");
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
=======
    let ports = available_ports()?
        .into_iter()
        .filter(|info| match &info.port_type {
            SerialPortType::UsbPort(usb_port_info) => {
                usb_port_info.vid == DEV_KIT_C_VENDOR_ID
                    || usb_port_info.vid == ESP_PROG_2_VENDOR_ID
            }
            _ => false,
        })
        .collect::<Vec<_>>();
    if ports.is_empty() {
        return Err(eyre!(
            "No ESP devices detected. Please plug one in and run this program again."
        ));
    }
    let stdout = io::stdout();
    {
        let mut out = stdout.lock();
        writeln!(out, "Detected an ESP device on: {ports:#?}")?;
        write!(out, "Please choose a \"port_name\" to connect to: ")?;
        out.flush()?;
    }
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

>>>>>>> main
    let client = HostClient::try_new_serial_cobs(
        buffer.trim(),
        ERROR_PATH,
        TX_QUEUE_SIZE,
        BAUD_RATE,
        VAR_SEQUENCE_KIND,
    )
    .map_err(|err| eyre!("Failed to initialize USB connection: {}", err))?;
<<<<<<< HEAD
=======

>>>>>>> main
    let terminal = ratatui::init();
    let result = App::new(client).await?.run(terminal).await;
    ratatui::restore();
    result
}

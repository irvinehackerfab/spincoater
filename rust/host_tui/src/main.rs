//! This crate provides a TUI for the PC connecting to the spincoater's ESP32.
pub mod app;

use std::io;

use color_eyre::{Result, eyre::eyre};
use postcard_rpc::{header::VarSeqKind, host_client::HostClient};
use sc_messages::icd::BAUD_RATE;
use tokio_serial::available_ports;

use crate::app::App;

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
    let client = HostClient::try_new_serial_cobs(
        buffer.trim(),
        ERROR_PATH,
        TX_QUEUE_SIZE,
        BAUD_RATE,
        VAR_SEQUENCE_KIND,
    )
    .map_err(|err| eyre!("Failed to initialize USB connection: {}", err))?;
    let terminal = ratatui::init();
    let result = App::new(client).await?.run(terminal).await;
    ratatui::restore();
    result
}

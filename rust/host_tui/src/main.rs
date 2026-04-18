//! This crate provides a TUI for the PC connecting to the spincoater's ESP32.
pub mod app;

use color_eyre::{Result, eyre::eyre};
use postcard_rpc::{header::VarSeqKind, host_client::HostClient};
use sc_messages::icd::BAUD_RATE;

use crate::app::App;

/// The URI that the MCU can use to report "unrecognized request" errors.
const ERROR_PATH: &str = "error";

// /// The product string of the MCU's USB port.
// const PRODUCT_STRING: &str = "CP2102N USB to UART Bridge Controller";

/// The path to the MCU's serial device.
const SERIAL_PATH: &str = "/dev/ttyUSB2";

/// The size of the outgoing queue.
const TX_QUEUE_SIZE: usize = 128;

/// The size of sequuence numbers used when making requests.
///
/// [`postcard_rpc`] gives no hint as to what this should be.
const VAR_SEQUENCE_KIND: VarSeqKind = VarSeqKind::Seq2;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    // Todo: Make this work with nusb somehow
    let client = HostClient::try_new_serial_cobs(
        SERIAL_PATH,
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

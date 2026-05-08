//! This module contains functionality for the terminal on the display.
pub mod channel;
pub mod ui;

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Receiver};
use mousefood::{EmbeddedBackend, prelude::Rgb565};
use postcard_rpc::server::{Sender, ServerError};
use ratatui::Terminal;
use static_cell::StaticCell;

use crate::{
    gpio::display::{
        DisplayType,
        terminal::channel::{TERMINAL_CHANNEL_SIZE, TuiEvent},
    },
    rpc::{WireRx, WireTx},
};

/// The static cell for the terminal.
///
/// [`update_terminal`] needs to borrow the terminal because passing it by value would copy 348 bytes.
pub static TERMINAL: StaticCell<Terminal<EmbeddedBackend<DisplayType, Rgb565>>> = StaticCell::new();

/// The state of the terminal.
#[derive(Default)]
pub struct TerminalState {
    /// The last error message reported by the server.
    server_error: Option<ServerError<WireTx, WireRx>>,
}

/// This task updates the terminal whenever another task requests it to.
#[embassy_executor::task]
pub async fn update_terminal(
    terminal: &'static mut Terminal<EmbeddedBackend<'static, DisplayType, Rgb565>>,
    to_server: Sender<WireTx>,
    from_all: Receiver<'static, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
) -> ! {
    let mut state = TerminalState::default();
    loop {
        if let Err(_err) = terminal.draw(|frame| state.draw(frame)) {
            let _ = to_server.log_str("Display error!").await;
        }
        match from_all.receive().await {
            TuiEvent::ServerError(server_error) => state.server_error = Some(server_error),
        }
    }
}

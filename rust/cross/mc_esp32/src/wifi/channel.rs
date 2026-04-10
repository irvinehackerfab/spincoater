//! This module contains functionality for sending data to and from the wifi connection.

use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Sender, TrySendError},
};
use sc_messages::{Command, Info};
use static_cell::ConstStaticCell;

use crate::gpio::display::terminal::channel::{ChannelKind, TERMINAL_CHANNEL_SIZE, TuiEvent};

/// The maximum number of messages allowed at a time in each channel to/from the message handler.
pub const HANDLER_CHANNEL_SIZE: usize = 2;
/// Used for passing commands from the socket to the command handler.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
pub static RECV_CMD_CHANNEL: ConstStaticCell<Channel<NoopRawMutex, Command, HANDLER_CHANNEL_SIZE>> =
    ConstStaticCell::new(Channel::new());

/// Used for passing info to the socket.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
pub static SEND_INFO_CHANNEL: ConstStaticCell<Channel<NoopRawMutex, Info, HANDLER_CHANNEL_SIZE>> =
    ConstStaticCell::new(Channel::new());

// Todo: Once you stop getting these errors and have finalized your capacities,
// remove this function to save stack space.
/// Tries to send a [`Command`] on a channel without blocking.
///
/// If sending fails,
/// sends the [`Command`] and sends a
/// [`ChannelFull`](crate::gpio::display::terminal::channel::TuiEvent::ChannelFull)
/// event to the terminal with the given [`ChannelKind`].
pub async fn send_cmd_or_report(
    sender: &Sender<'_, NoopRawMutex, Command, HANDLER_CHANNEL_SIZE>,
    command: Command,
    to_terminal: &Sender<'_, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
    channel_kind: ChannelKind,
) {
    if let Err(TrySendError::Full(message)) = sender.try_send(command) {
        to_terminal.send(TuiEvent::ChannelFull(channel_kind)).await;
        sender.send(message).await;
    }
}

// Todo: Once you stop getting these errors and have finalized your capacities,
// remove this function to save stack space.
/// Tries to send [`Info`] on a channel without blocking.
///
/// If sending fails,
/// sends the [`Info`] and sends a
/// [`ChannelFull`](crate::gpio::display::terminal::channel::TuiEvent::ChannelFull)
/// event to the terminal with the given [`ChannelKind`].
pub async fn send_info_or_report(
    sender: &Sender<'_, NoopRawMutex, Info, HANDLER_CHANNEL_SIZE>,
    message: Info,
    to_terminal: &Sender<'_, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
    channel_kind: ChannelKind,
) {
    if let Err(TrySendError::Full(message)) = sender.try_send(message) {
        to_terminal.send(TuiEvent::ChannelFull(channel_kind)).await;
        sender.send(message).await;
    }
}

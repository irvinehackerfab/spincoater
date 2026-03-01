//! This module contains functionality for sending data to and from the wifi connection.

use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Sender, TrySendError},
};
use sc_messages::Message;
use static_cell::ConstStaticCell;

use crate::gpio::display::terminal::channel::{ChannelKind, TERMINAL_CHANNEL_SIZE, TuiEvent};

/// The maximum number of messages allowed at a time in each channel to/from the message handler.
pub const HANDLER_CHANNEL_SIZE: usize = 2;
/// Used for passing messages from the socket to the message handler.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
/// This does not use a zerocopy channel because [`Message`] is currently smaller than a reference.
pub static RECV_MSG_CHANNEL: ConstStaticCell<Channel<NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>> =
    ConstStaticCell::new(Channel::new());

/// Used for passing messages from the message handler to the socket.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
/// This does not use a zerocopy channel because [`Message`] is currently smaller than a reference.
pub static SEND_MSG_CHANNEL: ConstStaticCell<Channel<NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>> =
    ConstStaticCell::new(Channel::new());

// Todo: Once you stop getting these errors and have finalized your capacities,
// remove this function to save stack space.
/// Tries to send a [`Message`] on a channel without blocking.
///
/// If sending fails,
/// sends the [`Message`] and sends a
/// [`ChannelFull`](crate::gpio::display::terminal::channel::TuiEvent::ChannelFull)
/// event to the terminal with the given [`ChannelKind`].
pub async fn send_msg_or_report(
    to_msg_handler: &Sender<'_, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
    message: Message,
    to_terminal: &Sender<'_, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
    channel_kind: ChannelKind,
) {
    if let Err(TrySendError::Full(message)) = to_msg_handler.try_send(message) {
        to_terminal.send(TuiEvent::ChannelFull(channel_kind)).await;
        to_msg_handler.send(message).await;
    }
}

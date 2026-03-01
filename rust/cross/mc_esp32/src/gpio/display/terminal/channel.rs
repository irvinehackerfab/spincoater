//! This module contains functionality for sending data to the terminal.

use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Sender, TrySendError},
};
use static_cell::ConstStaticCell;

use crate::wifi::{ApState, tcp::SocketState};

/// The maximum number of messages allowed at a time in each channel to/from the terminal.
pub const TERMINAL_CHANNEL_SIZE: usize = 8;
/// Used for passing messages from the wifi and socket handler to the terminal.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
/// This does not use a zerocopy channel because [`TuiEvent`] is currently the same size as a reference.
pub static TERMINAL_CHANNEL: ConstStaticCell<
    Channel<NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
> = ConstStaticCell::new(Channel::new());

/// All possible messages sent to the terminal.
#[derive(Debug)]
pub enum TuiEvent {
    /// The wifi state changed.
    WifiEvent(ApState),
    /// The socket state changed.
    SocketEvent(SocketState),
    /// The PWM output duty cycle changed.
    DutyChanged(u16),
    /// A value for plate revolutions per minute has been calculated.
    RpmValue(u16),
    /// A channel was found to be full.
    ChannelFull(ChannelKind),
}

/// Information about the [`Channel`]s.
#[derive(Debug, Default)]
pub struct ChannelStatus {
    /// Whether [`crate::wifi::channel::RECV_MSG_CHANNEL`] was ever full.
    /// If ever becomes true, [`crate::wifi::channel::HANDLER_CHANNEL_SIZE`] needs to be increased.
    pub recv_msg_channel_was_full: bool,
    /// Whether [`crate::wifi::channel::SEND_MSG_CHANNEL`] was ever full.
    /// If ever becomes true, [`crate::wifi::channel::HANDLER_CHANNEL_SIZE`] needs to be increased.
    pub send_msg_channel_was_full: bool,
    /// Whether the [`TERMINAL_CHANNEL`] was ever full.
    /// If this is ever true, [`TERMINAL_CHANNEL_SIZE`] needs to be increased.
    pub terminal_channel_was_full: bool,
}

impl ChannelStatus {
    /// Set the requested "channel full" status to `true`.
    pub fn set_full(&mut self, channel_kind: ChannelKind) {
        match channel_kind {
            ChannelKind::RecvMsg => self.recv_msg_channel_was_full = true,
            ChannelKind::SendMsg => self.send_msg_channel_was_full = true,
            ChannelKind::Terminal => self.terminal_channel_was_full = true,
        }
    }
}

/// All channels we use.
#[derive(Debug, Clone, Copy)]
pub enum ChannelKind {
    /// [`crate::wifi::channel::RECV_MSG_CHANNEL`]
    RecvMsg,
    /// [`crate::wifi::channel::SEND_MSG_CHANNEL`]
    SendMsg,
    /// [`TERMINAL_CHANNEL`]
    Terminal,
}

// Todo: Once you stop getting these errors and have finalized your capacities,
// remove this function to save stack space.
/// Tries to send a [`TuiEvent`] on a channel without blocking.
///
/// If sending fails,
/// sends both the [`TuiEvent`] and a [`TuiEvent::ChannelFull`]
/// to the terminal.
pub async fn send_event_or_report(
    to_terminal: &Sender<'_, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
    event: TuiEvent,
) {
    if let Err(TrySendError::Full(event)) = to_terminal.try_send(event) {
        to_terminal
            .send(TuiEvent::ChannelFull(ChannelKind::Terminal))
            .await;
        to_terminal.send(event).await;
    }
}

//! This module contains functionality for sending data to the terminal.

use crate::runners::rpm::channel::RunAt;
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Receiver, Sender},
};
use sc_messages::touchscreen::TouchPoint;
use static_cell::ConstStaticCell;

/// The maximum number of messages allowed at a time in each channel to/from the terminal.
pub const TERMINAL_CHANNEL_SIZE: usize = 8;
/// Used for passing messages to the terminal.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
/// This does not use a zerocopy channel because [`TuiEvent`] is cheap to copy.
pub static TERMINAL_CHANNEL: ConstStaticCell<
    Channel<NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
> = ConstStaticCell::new(Channel::new());

/// The type of the terminal channel sender.
pub type TerminalSender = Sender<'static, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>;

/// The type of the terminal channel receiver.
pub type TerminalReceiver = Receiver<'static, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>;

/// All possible messages sent to the terminal.
pub enum TuiEvent {
    /// The screen was touched.
    Touch(TouchPoint),
    /// The runner sent an update.
    Runner(RunAt),
    /// The runner finished.
    RunnerFinished,
}

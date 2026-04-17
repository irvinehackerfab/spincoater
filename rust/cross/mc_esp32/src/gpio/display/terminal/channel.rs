//! This module contains functionality for sending data to the terminal.

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use postcard_rpc::server::ServerError;
use static_cell::ConstStaticCell;

use crate::rpc::{WireRx, WireTx};

/// The maximum number of messages allowed at a time in each channel to/from the terminal.
pub const TERMINAL_CHANNEL_SIZE: usize = 8;
/// Used for passing messages from the wifi and socket handler to the terminal.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
/// This does not use a zerocopy channel because [`TuiEvent`] is cheap to copy.
pub static TERMINAL_CHANNEL: ConstStaticCell<
    Channel<NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
> = ConstStaticCell::new(Channel::new());

/// All possible messages sent to the terminal.
pub enum TuiEvent {
    ServerError(ServerError<WireTx, WireRx>),
}

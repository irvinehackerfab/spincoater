//! This module contains the channel functionality for the RPM runner.

use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Receiver, Sender},
};
use static_cell::ConstStaticCell;

/// The maximum number of messages allowed at a time in each channel to/from the terminal.
pub const RUNNER_CHANNEL_SIZE: usize = 1;
/// Used for passing messages to the terminal.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
/// This does not use a zerocopy channel because [`RunRequest`] is cheap to copy.
pub static RUNNER_CHANNEL: ConstStaticCell<
    Channel<NoopRawMutex, RunnerRequest, RUNNER_CHANNEL_SIZE>,
> = ConstStaticCell::new(Channel::new());

/// The type of the runner channel sender.
pub type RunnerSender = Sender<'static, NoopRawMutex, RunnerRequest, RUNNER_CHANNEL_SIZE>;

/// The type of the runner channel receiver.
pub type RunnerReceiver = Receiver<'static, NoopRawMutex, RunnerRequest, RUNNER_CHANNEL_SIZE>;

/// The message types sent between the terminal and runner.
pub enum RunnerRequest {
    Run(RunAt),
    Stop,
}

/// The rpm and time.
#[derive(Debug)]
pub struct RunAt {
    /// Plate RPM.
    pub rpm: u16,
    /// Time in seconds.
    pub time: u16,
}

impl RunAt {
    /// Creates a run request.
    ///
    /// RPM should be plate RPM and time should be seconds.
    #[must_use]
    pub fn new(rpm: u16, time: u16) -> Self {
        Self { rpm, time }
    }
}

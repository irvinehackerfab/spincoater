use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

use crate::motion_profile::Setpoint;

/// Messages from the host PC to the microcontroller.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Schema)]
pub enum Command {
    /// Add a setpoint to the motion profile.
    ///
    /// The MCU will only listen to this while disabled.
    Add(Setpoint),
    /// Execute the motion profile.
    Start,
    /// Stop the motion profile and discard it.
    Stop,
}

/// The possible reasons why the MCU might refuse a command.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Schema)]
pub enum CommandRefused {
    /// A motion profile is running.
    Running,
    /// No motion profile is running.
    NotRunning,
}

/// See [this issue](https://github.com/jamesmunns/postcard-rpc/issues/56) for why we need a type alias.
pub type CommandResult = Result<(), CommandRefused>;

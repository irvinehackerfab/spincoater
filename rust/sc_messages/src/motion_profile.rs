use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

use crate::pwm::DutyCycle;

/// The maximum allowed number of setpoints in a single motion profile.
///
/// Any further setpoints will be ignored by the microcontroller.
pub const MAX_SETPOINTS: usize = 127;

/// A single target motor RPM value with the corresponding time taken to reach that RPM.
///
/// These setpoints are combined to create a motion profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Schema)]
pub struct Setpoint {
    /// The target motor RPM.
    pub rpm: u16,
    /// The time (in micros) for this rpm.
    /// The MCU expects this to be time since the start of the motion profile.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    #[serde(rename = "time (micros)")]
    pub time: u64,
}

/// The current state of the motion profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Schema)]
pub struct State {
    /// The setpoint motor RPM.
    pub setpoint_rpm: u16,
    /// The measured motor RPM.
    pub current_rpm: u16,
    /// The current duty cycle being set to try and reach the setpoint.
    pub duty_cycle: DutyCycle,
    /// The time (in micros) since the motion profile started.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    pub time: u64,
}

/// Motion profile messages from the host PC to the microcontroller.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Schema)]
pub enum Request {
    /// Add a setpoint to the motion profile.
    ///
    /// The MCU will only accept this while disabled.
    Add(Setpoint),
    /// Clear all setpoints.
    ///
    /// The MCU will only accept this while disabled.
    ClearSetpoints,
    /// Execute the motion profile.
    ///
    /// The MCU will only accept this while disabled.
    Start,
    /// Stop the motion profile and discard it.
    ///
    /// The MCU will only accept this while enabled.
    Stop,
}

/// The possible reasons why the MCU might refuse a command.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Schema)]
pub enum RequestRefused {
    /// The host PC sent too many setpoints.
    TooManySetpoints,
    /// The host PC sent a setpoint with a time that is less than the last one that was sent.
    IncorrectSetpointOrder,
    /// A motion profile is running.
    Running,
    /// No motion profile is running.
    NotRunning,
}

/// See [this issue](https://github.com/jamesmunns/postcard-rpc/issues/56) for why we need a type alias.
pub type RequestResult = Result<(), RequestRefused>;

/// See [this issue](https://github.com/jamesmunns/postcard-rpc/issues/56) for why we need a type alias.
pub type StateOrDisabled = Option<State>;

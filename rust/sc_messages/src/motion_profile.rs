use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

use crate::pwm::DutyCycle;

/// The maximum allowed number of setpoints in a single motion profile.
///
/// Any further setpoints will be ignored by the microcontroller.
pub const MAX_SETPOINTS: usize = 127;

/// A single target plate RPM value with the corresponding time taken to reach that RPM.
///
/// These setpoints are combined to create a motion profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Schema)]
pub struct Setpoint {
    /// The target plate RPM.
    pub rpm: u16,
    /// The time (in micros) that should be taken to reach the rpm.
    /// The MCU expects this to be time since last setpoint,
    /// and it sends it back to the host PC as time since the start of the motion profile.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    #[serde(rename = "time (micros)")]
    pub time: u64,
}

/// The current state of the motion profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Schema)]
pub struct State {
    /// The setpoint plate RPM.
    pub setpoint_rpm: u16,
    /// The measured plate RPM.
    pub current_rpm: u16,
    /// The current duty cycle being set to try and reach the setpoint.
    pub duty_cycle: DutyCycle,
    /// The time (in micros) since the motion profile started.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    #[serde(rename = "time (micros)")]
    pub time: u64,
}

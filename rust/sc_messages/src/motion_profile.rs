use core::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::pwm::DutyCycle;

/// The maximum allowed number of setpoints in a single motion profile.
///
/// Any further setpoints will be ignored by the microcontroller.
pub const MAX_SETPOINTS: usize = 127;

/// All message types sent from the MCU to the host PC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McuMessage {
    /// The MCU has state to report.
    ///
    /// The MCU will send this while running.
    State(State),
    /// The spincoater finished the motion profile.
    Finished,
}

/// All messages types sent from the host PC to the MCU.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HostMessage {
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

/// A single target motor RPM value with the corresponding time taken to reach that RPM.
///
/// These setpoints are combined to create a motion profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Setpoint {
    /// The target motor RPM.
    pub rpm: u16,
    /// The time (in micros) since the start of the motion profile.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    #[serde(rename = "time (micros)")]
    pub time: u64,
}

/// Setpoints are ordered by time.
impl Ord for Setpoint {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
    }
}

/// Setpoints are ordered by time.
impl PartialOrd for Setpoint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The current state of the motion profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct State {
    /// The setpoint motor RPM.
    pub setpoint_rpm: u16,
    /// The measured motor RPM.
    pub current_rpm: u16,
    /// Setpoint RPM - current RPM.
    pub rpm_error: i16,
    /// The current duty cycle being set to try and reach the setpoint.
    pub duty_cycle: DutyCycle,
    /// The time (in micros) since the motion profile started.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    pub time: u64,
}

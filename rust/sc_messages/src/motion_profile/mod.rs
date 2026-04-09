use serde::{Deserialize, Serialize};

/// The maximum allowed number of setpoints in a single motion profile.
///
/// Any further setpoints will be ignored by the microcontroller.
pub const MAX_SETPOINTS: usize = 128;

/// A single target plate RPM value with the corresponding time taken to reach that RPM.
///
/// These setpoints are combined to create a motion profile.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Setpoint {
    /// The target plate RPM.
    pub rpm: u16,
    /// The time (in ticks) that should be taken to reach the rpm.
    /// The MCU expects this to be time since last setpoint,
    /// and it internally converts it to time since the start of the motion profile.
    ///
    /// The host PC is expected to know the tick rate of the microcontroller.
    /// See `host_tui`'s Cargo.toml for more info.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    #[serde(rename = "time (ticks)")]
    pub time: u64,
}

/// A measured plate RPM value with the time it was measured.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct State {
    /// The measured plate RPM.
    pub rpm: u16,
    /// The time (in ticks) since the motion profile started.
    ///
    /// The host PC is expected to know the tick rate of the microcontroller.
    /// See `host_tui`'s Cargo.toml for more info.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    #[serde(rename = "time (ticks)")]
    pub time: u64,
}

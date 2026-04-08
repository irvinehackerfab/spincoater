use serde::{Deserialize, Serialize};

/// A single target plate RPM value with the corresponding deadline for reaching that RPM.
///
/// These setpoints are combined to create a motion profile.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Setpoint {
    /// The target plate RPM.
    pub rpm: u16,
    /// The time (in ticks) that should be taken to reach the rpm.
    ///
    /// The host PC is expected to know the tick rate of the microcontroller.
    /// See `host_tui`'s Cargo.toml for more info.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    pub after: u64,
}

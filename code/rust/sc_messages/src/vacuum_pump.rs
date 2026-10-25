use serde::{Deserialize, Serialize};

/// All message types sent from the host PC to the MCU.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HostMessage {
    /// Enable the vacuum pump.
    Enable,
    /// Disable the vacuum pump.
    Disable,
}

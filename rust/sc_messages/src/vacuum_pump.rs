use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

/// Vacuum pump messages from the host PC to the microcontroller.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Schema)]
pub enum Request {
    /// Enable the vacuum pump.
    Enable,
    /// Disable the vacuum pump.
    Disable,
}

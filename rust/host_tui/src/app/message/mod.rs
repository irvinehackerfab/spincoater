//! This module contains types that wrap the types in [`sc_messages`].

use cfg_if::cfg_if;
use chrono::{DateTime, Local};
use sc_messages::Message;
use std::fmt::{Display, Formatter};

/// A message, with the time it was received.
#[derive(Debug, Clone)]
pub struct MessageInfo {
    pub message: Message,
    pub timestamp: DateTime<Local>,
    pub from_mcu: bool,
}

impl MessageInfo {
    #[must_use]
    pub fn new(message: Message, from_mcu: bool) -> Self {
        Self {
            message,
            timestamp: Local::now(),
            from_mcu,
        }
    }
}

cfg_if! {
    if #[cfg(feature = "dev-socket")] {
        impl Display for MessageInfo {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{} (fake socket) -- {}: {}",
                    self.timestamp.format("%m-%d-%Y %H:%M:%S"),
                    if self.from_mcu { "From MCU" } else { "To MCU" },
                    self.message
                )
            }
        }
    } else {
        impl Display for MessageInfo {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{} -- {}: {}",
                    self.timestamp.format("%m-%d-%Y %H:%M:%S"),
                    if self.from_mcu { "From MCU" } else { "To MCU" },
                    self.message
                )
            }
        }
    }
}

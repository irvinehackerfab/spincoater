//! The [interface control document](https://en.wikipedia.org/wiki/Interface_control_document) for the microcontrollers and host PC.
//!
//! UART requirements:
//! - Even parity
//! - 8 data bits
//! - 1 stop bit
//! - No flow control
//! - COBS encoding

use serde::{Deserialize, Serialize};

use crate::{
    motion_profile::{self},
    vacuum_pump::{self},
};

/// The baud rate for UART communication.
///
/// This value was taken from [`esp_hal::uart::Config::default`]
/// and is placed here so [`esp_hal::uart::Config::default`] doesn't change it under our feet.
pub const BAUD_RATE: u32 = 115_200;

/// The max [COBS](https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing)-serialized size of an [`McuMessage`].
///
/// An [`McuMessage`] is guaranteed to take up at most this many bytes in a buffer.
pub const MAX_MCU_MESSAGE_SIZE: usize = 64;

/// The max [COBS](https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing)-serialized size of a [`HostMessage`].
///
/// An [`HostMessage`] is guaranteed to take up at most this many bytes in a buffer.
pub const MAX_HOST_MESSAGE_SIZE: usize = MAX_MCU_MESSAGE_SIZE;

/// All message types sent from the MCU to the host PC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McuMessage {
    MotionProfile(motion_profile::McuMessage),
}

/// All message types sent from the host PC to the MCU.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HostMessage {
    MotionProfile(motion_profile::HostMessage),
    VacuumPump(vacuum_pump::HostMessage),
    /// The host is disconnecting.
    Disconnecting,
}

#[cfg(test)]
mod test {
    use postcard::to_vec_cobs;

    use crate::{
        motion_profile::{Setpoint, State},
        pwm::DutyCycle,
    };

    use super::*;

    /// Proves that [`MAX_MCU_MESSAGE_SIZE`] is valid for all [`McuMessage`].
    #[test]
    fn max_mcu_message_size() {
        let message = McuMessage::MotionProfile(motion_profile::McuMessage::Finished);
        to_vec_cobs::<_, MAX_MCU_MESSAGE_SIZE>(&message)
            .expect("Failed to fit `Finished` in `MAX_MCU_MESSAGE_SIZE` bytes");

        let message = McuMessage::MotionProfile(motion_profile::McuMessage::State(State {
            setpoint_rpm: 1000,
            current_rpm: 1000,
            rpm_error: 100,
            duty_cycle: DutyCycle::from(60000u16),
            time: 100_000,
        }));
        to_vec_cobs::<_, MAX_MCU_MESSAGE_SIZE>(&message)
            .expect("Failed to fit `State` in `MAX_MCU_MESSAGE_SIZE` bytes");
    }

    /// Proves that [`MAX_HOST_MESSAGE_SIZE`] is valid for all [`HostMessage`].
    #[test]
    fn max_host_message_size() {
        let message = HostMessage::Disconnecting;
        to_vec_cobs::<_, MAX_HOST_MESSAGE_SIZE>(&message)
            .expect("Failed to fit `Finished` in `MAX_MCU_MESSAGE_SIZE` bytes");

        let message = HostMessage::MotionProfile(motion_profile::HostMessage::Add(Setpoint {
            rpm: 10000,
            time: 100_000,
        }));
        to_vec_cobs::<_, MAX_HOST_MESSAGE_SIZE>(&message)
            .expect("Failed to fit `State` in `MAX_MCU_MESSAGE_SIZE` bytes");

        let message = HostMessage::VacuumPump(vacuum_pump::HostMessage::Enable);
        to_vec_cobs::<_, MAX_HOST_MESSAGE_SIZE>(&message)
            .expect("Failed to fit `State` in `MAX_MCU_MESSAGE_SIZE` bytes");
    }
}

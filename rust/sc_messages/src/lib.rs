//! This cross-platform crate describes the message types sent between the host PC and microcontrollers.
#![no_std]
#[cfg(feature = "std")]
extern crate std;

pub mod motion_profile;

use core::{
    fmt::{self, Display, Formatter},
    ops::Deref,
    result::Result,
};
use serde::{Deserialize, Serialize};

use crate::motion_profile::Setpoint;

/// The value corresponding to 100% of the PWM period.
/// See [`../cross/mc_esp32/src/pwm/mod.rs`] for an explanation on the choice for this value.
pub const PERIOD: u16 = u16::MAX - 1_535;

/// The current motor controller reads 10% of [`PERIOD`] as 100% power.
pub const MAX_POWER_DUTY: DutyCycle = DutyCycle(PERIOD / 10);

/// The current motor controller reads 5% of [`PERIOD`] as 0% power.
pub const STOP_DUTY: DutyCycle = DutyCycle(PERIOD / 20);

/// A duty cycle.
/// 0-100% is encoded as 0..[`PERIOD`].
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct DutyCycle(u16);

impl Deref for DutyCycle {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for DutyCycle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The [`u16`]'s value was too high to be considered a [`DutyCycle`].
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub struct OutOfRange(pub u16);

impl Display for OutOfRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} is greater than the maximum duty cycle of {}.",
            self.0, PERIOD
        )
    }
}

impl TryFrom<u16> for DutyCycle {
    type Error = OutOfRange;

    /// Attempt to wrap a [`u16`] in [`DutyCycle`].
    ///
    /// This fails if the value is greater than [`PERIOD`].
    fn try_from(value: u16) -> Result<DutyCycle, OutOfRange> {
        if value <= PERIOD {
            Ok(Self(value))
        } else {
            Err(OutOfRange(value))
        }
    }
}

/// Messages from the host PC to the microcontroller.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

// Messages from the microcontroller to the host PC.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Info {
    /// The current setpoint of the motion profile.
    Setpoint(motion_profile::Setpoint),
    /// The current state of the motion profile.
    State(motion_profile::State),
    /// The current duty cycle.
    DutyCycle(DutyCycle),
}

#[cfg(test)]
mod test {
    use std::println;

    use super::*;
    use postcard::{from_bytes_cobs, to_vec_cobs};

    /// Keep this up to date with `../cross/mc_esp32/src/wifi/tcp/mod.rs` `BUFFER_SIZE`
    const BUFFER_SIZE: usize = 64;

    /// Ensures that a COBS encoded command fits in the buffer size used by all spin coater programs.
    ///
    /// This test guarantees that we can use the [read_with](https://docs.embassy.dev/embassy-net/git/default/tcp/struct.TcpSocket.html#method.read_with)
    /// and [write_with](https://docs.embassy.dev/embassy-net/git/default/tcp/struct.TcpSocket.html#method.write_with) methods.
    #[test]
    fn test_fits_in_buffer() {
        for rpm in [0, 10, u16::MAX] {
            for ticks in [0, 10, u64::MAX] {
                let setpoint = Setpoint { rpm, time: ticks };
                let command = Command::Add(setpoint);
                let mut send =
                    to_vec_cobs::<Command, BUFFER_SIZE>(&command).expect("Failed to serialize");
                println!("Cobs message is {} bytes long.", send.len());
                let output = from_bytes_cobs::<Command>(&mut send);
                assert!(
                    matches!(output, Ok(Command::Add(Setpoint { rpm: out_rpm, time: out_ticks })) if out_rpm == rpm && out_ticks == ticks)
                );
            }
        }
    }
}

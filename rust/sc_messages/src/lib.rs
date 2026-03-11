//! This cross-platform crate describes the message types sent between the host PC and microcontrollers.
#![no_std]
#[cfg(feature = "std")]
extern crate std;

use core::{
    fmt::{self, Display, Formatter},
    ops::Deref,
    result::Result,
};
use serde::{Deserialize, Serialize};

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

/// Messages between the host PC and the microcontroller.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Message {
    /// From PC to MCU: Set a duty cycle.
    ///
    /// From MCU to PC: The current duty cycle.
    DutyCycle(DutyCycle),
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Message::DutyCycle(duty) => write!(f, "Set duty cycle to: {}", duty.0),
        }
    }
}

#[cfg(test)]
mod test {
    use std::println;

    use super::*;
    use postcard::{Error, from_bytes, from_bytes_cobs, to_vec, to_vec_cobs};

    /// Keep this up to date with `../cross/mc_esp32/src/wifi/tcp/mod.rs` `BUFFER_SIZE`
    const BUFFER_SIZE: usize = 64;

    /// The correct message must be obtainable from multiple deserialization attempts on the same buffer.
    ///
    /// This test guarantees that we can use the [read](https://docs.embassy.dev/embassy-net/git/default/tcp/struct.TcpSocket.html#method.read) method.
    #[test]
    fn test_re_deserialize() {
        for duty in 0..u16::MAX {
            let duty = DutyCycle(duty);
            let msg = Message::DutyCycle(duty);
            let send = to_vec::<Message, BUFFER_SIZE>(&msg).expect("Failed to serialize");
            for (i, _) in send
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != 0 && *i < send.len())
            {
                let send_clone = send.clone();
                let error = from_bytes::<Message>(&send_clone[..i]);
                assert!(matches!(error, Err(Error::DeserializeUnexpectedEnd)));
                let output = from_bytes::<Message>(&send_clone);
                assert!(matches!(output, Ok(Message::DutyCycle(out_duty)) if out_duty == duty));
            }
        }
    }

    /// Ensures that a COBS encoded message fits in the buffer size used by all spin coater programs.
    ///
    /// This test guarantees that we can use the [read_with](https://docs.embassy.dev/embassy-net/git/default/tcp/struct.TcpSocket.html#method.read_with)
    /// and [write_with](https://docs.embassy.dev/embassy-net/git/default/tcp/struct.TcpSocket.html#method.write_with) methods.
    #[test]
    fn test_fits_in_buffer() {
        for duty in 0..u16::MAX {
            let duty = DutyCycle(duty);
            let msg = Message::DutyCycle(duty);
            let mut send = to_vec_cobs::<Message, BUFFER_SIZE>(&msg).expect("Failed to serialize");
            println!("Cobs message is {} bytes long.", send.len());
            let output = from_bytes_cobs::<Message>(&mut send);
            assert!(matches!(output, Ok(Message::DutyCycle(out_duty)) if out_duty == duty));
        }
    }
}

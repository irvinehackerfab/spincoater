#![no_std]
#[cfg(feature = "std")]
extern crate std;

use core::fmt::{Display, Formatter, Result};
use serde::{Deserialize, Serialize};

/// The value corresponding to 100% duty cycle.
/// See [`../cross/mc_esp32/src/pwm/mod.rs`] for an explanation on the choice for this value.
pub const MAX_DUTY: u16 = u16::MAX - 1_536;

/// Messages between the host PC and the microcontroller.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Message {
    /// From PC to MCU: Set a duty cycle.
    ///
    /// From MCU to PC: The current duty cycle.
    DutyCycle(u16),
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Message::DutyCycle(duty) => write!(f, "Set duty cycle to: {duty}"),
        }
    }
}

#[cfg(test)]
mod test {
    use std::println;

    use super::*;
    use postcard::{Error, from_bytes, from_bytes_cobs, to_vec, to_vec_cobs};

    // Keep this up to date with ../cross/mc_esp32/src/bin/wifi_pwm/wifi/mod.rs BUFFER_SIZE
    const BUFFER_SIZE: usize = 64;

    /// The correct message must be obtainable from multiple deserialization attempts on the same buffer.
    ///
    /// This test guarantees that we can use the [read](https://docs.embassy.dev/embassy-net/git/default/tcp/struct.TcpSocket.html#method.read) method.
    #[test]
    fn test_re_deserialize() {
        for duty in 0..u16::MAX {
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
            let msg = Message::DutyCycle(duty);
            let mut send = to_vec_cobs::<Message, BUFFER_SIZE>(&msg).expect("Failed to serialize");
            println!("Cobs message is {} bytes long.", send.len());
            let output = from_bytes_cobs::<Message>(&mut send);
            assert!(matches!(output, Ok(Message::DutyCycle(out_duty)) if out_duty == duty));
        }
    }
}

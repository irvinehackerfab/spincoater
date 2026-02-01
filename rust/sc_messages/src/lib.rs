#![no_std]
#[cfg(feature = "std")]
extern crate std;

use core::fmt::{Display, Formatter, Result};
use serde::{Deserialize, Serialize};

/// Messages between the host PC and the microcontroller.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Message {
    /// From PC to MCU: Set a duty cycle between 0 and 99.
    ///
    /// From MCU to PC: The current duty cycle between 0 and 99.
    DutyCycle(u8),
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Message::DutyCycle(duty) => write!(f, "Set duty cycle to: {}", duty),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use postcard::{Error, from_bytes, to_vec};

    /// The correct message must be obtainable from multiple deserialization attempts on the same buffer.
    #[test]
    fn test_re_deserialize() {
        for duty in 0..u8::MAX {
            let msg = Message::DutyCycle(duty);
            let send = to_vec::<Message, 64>(&msg).unwrap();
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
}

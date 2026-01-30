#![cfg_attr(not(test), no_std)]

use serde::{Deserialize, Serialize};

/// Messages between the host PC and the microcontroller.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Message {
    /// Set a duty cycle between 0 and 99.
    SetDutyCycle(u8),
}

#[cfg(test)]
mod test {
    use super::*;
    use postcard::{Error, from_bytes, to_vec};

    /// The correct message must be obtainable from multiple deserialization attempts on the same buffer.
    #[test]
    fn test_re_deserialize() {
        for duty in 0..u8::MAX {
            let msg = Message::SetDutyCycle(duty);
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
                assert!(matches!(output, Ok(Message::SetDutyCycle(out_duty)) if out_duty == duty));
            }
        }
    }
}

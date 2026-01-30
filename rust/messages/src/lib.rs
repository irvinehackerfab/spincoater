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
    use postcard::{Error, from_bytes_cobs, to_vec_cobs};

    /// A message must be obtainable from multiple deserialization attempts.
    #[test]
    fn test_re_deserialize() {
        let msg = Message::SetDutyCycle(5);
        let mut send = to_vec_cobs::<Message, 64>(&msg).unwrap();
        assert!(send.len() > 1);
        let x = from_bytes_cobs::<Message>(&mut send[..1]);
        assert!(matches!(x, Err(Error::DeserializeUnexpectedEnd)));
        let x = from_bytes_cobs::<Message>(&mut send);
        assert!(matches!(x, Ok(Message::SetDutyCycle(5))));
    }
}

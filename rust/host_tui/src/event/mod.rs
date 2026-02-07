use crate::app::MessageInfo;
use bytes::{BufMut, BytesMut};
use color_eyre::{Result, eyre::OptionExt};
use futures::{FutureExt, StreamExt};
use postcard::from_bytes_cobs;
use ratatui::crossterm::event::Event as CrosstermEvent;
use sc_messages::Message;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::mpsc::{self, UnboundedSender},
};

// Keep this up to date with ../cross/mc_esp32/src/bin/wifi_pwm/wifi/mod.rs BUFFER_SIZE
pub const BUFFER_SIZE: usize = 64;

/// Representation of all possible events.
#[derive(Clone, Debug)]
pub enum TuiEvent {
    /// Crossterm events such as keyboard inputs.
    ///
    /// These events are emitted by the terminal.
    Crossterm(CrosstermEvent),
    /// Events from the microcontroller connection.
    Wireless(MessageInfo),
}

/// Terminal event handler.
#[derive(Debug)]
pub struct EventHandler {
    /// Event receiver channel.
    from_tasks: mpsc::UnboundedReceiver<Result<TuiEvent>>,
    to_mcu: OwnedWriteHalf,
    send_buffer: [u8; BUFFER_SIZE],
}

impl EventHandler {
    /// Constructs a new instance of [`EventHandler`] and spawns a new thread to handle events.
    pub fn new(stream: TcpStream) -> Self {
        let (from_mcu, to_mcu) = stream.into_split();
        let (to_handler, from_tasks) = mpsc::unbounded_channel();
        let to_handler_2 = to_handler.clone();
        // Spawn crossterm event handler.
        tokio::spawn(await_crossterm_events(to_handler));
        // Spawn stream message handler.
        tokio::spawn(await_stream_messages(from_mcu, to_handler_2));
        Self {
            from_tasks,
            to_mcu,
            send_buffer: [0u8; BUFFER_SIZE],
        }
    }

    /// Receives an event from the sender.
    ///
    /// This function blocks until an event is received.
    ///
    /// # Errors
    ///
    /// This function returns an error if the sender channel is disconnected. This can happen if an
    /// error occurs in the event thread. In practice, this should not happen unless there is a
    /// problem with the underlying terminal.
    pub async fn next(&mut self) -> Result<Result<TuiEvent>> {
        self.from_tasks
            .recv()
            .await
            .ok_or_eyre("Failed to receive event")
    }

    /// Sends a message to the MCU, and returns the message along with the time at which it finished sending.
    ///
    /// # Errors
    /// Returns an error if deserialization fails, or writing to the TCP socket fails.
    pub async fn send(&mut self, message: Message) -> Result<()> {
        let written_chunk = postcard::to_slice_cobs(&message, &mut self.send_buffer)?;
        self.to_mcu.write_all(written_chunk).await?;
        Ok(())
    }
}

async fn await_crossterm_events(to_handler: UnboundedSender<Result<TuiEvent>>) {
    let mut reader = crossterm::event::EventStream::new();
    loop {
        match reader.next().fuse().await {
            Some(result) => match result {
                Ok(event) => {
                    // If the channel is closed, this task is done.
                    if to_handler.send(Ok(TuiEvent::Crossterm(event))).is_err() {
                        return;
                    }
                }
                Err(error) => {
                    eprintln!("EventStream error: {error}");
                    // The errors are undocumented so I'm curious to see what they are.
                    panic!("See previous message")
                }
            },
            // If the stream is closed, this task is done.
            None => return,
        }
    }
}

/// Reads bytes into a buffer until a complete message is received and sends the message to the handler (and repeats forever).
///
/// The message must be [COBS encoded](https://docs.rs/postcard/latest/postcard/ser_flavors/struct.Cobs.html)
/// and must fit in [`BUFFER_SIZE`] bytes.
async fn await_stream_messages(
    mut from_mcu: OwnedReadHalf,
    to_handler: UnboundedSender<Result<TuiEvent>>,
) {
    let mut buffer = BytesMut::with_capacity(BUFFER_SIZE);
    loop {
        // BUFFER_SIZE is too small if we're filling up the buffer.
        assert!(buffer.has_remaining_mut());
        match from_mcu.read_buf(&mut buffer).await {
            // End of file
            Ok(0) => return,
            Ok(_) => {
                let mut written_chunk = buffer.split();
                // We must search for 0 before deserializing because from_bytes_cobs mutates the slice regardless of success.
                while let Some(idx) = written_chunk.iter().position(|byte| *byte == 0u8) {
                    let end = idx + 1;
                    let mut msg_chunk = written_chunk.split_to(end);
                    match from_bytes_cobs::<Message>(&mut msg_chunk) {
                        Ok(message) => {
                            // Send message
                            if to_handler
                                .send(Ok(TuiEvent::Wireless(MessageInfo::from(message))))
                                .is_err()
                            {
                                // If the channel is closed, this task is done.
                                return;
                            }
                        }
                        Err(error) => {
                            // If deserialization fails, the task is done.
                            let _ = to_handler.send(Err(error.into()));
                            return;
                        }
                    }
                    // Clear the written data so the buffer can be reused.
                    msg_chunk.clear();
                    written_chunk.unsplit(msg_chunk);
                }
                buffer.unsplit(written_chunk);
            }
            Err(error) => {
                // If reading fails, the task is done.
                let _ = to_handler.send(Err(error.into()));
                return;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Incremental writes like those in a TCP stream must work properly.
    #[test]
    fn test_incremental_writes() {
        let mut buf = BytesMut::with_capacity(BUFFER_SIZE);
        buf.put(&b"abc"[..]);
        let written_chunk = buf.split();
        assert!(buf.is_empty());
        assert_eq!(written_chunk, b"abc"[..]);
        buf.unsplit(written_chunk);
        buf.put(&b"def"[..]);
        let written_chunk = buf.split();
        assert_eq!(written_chunk, b"abcdef"[..]);
    }

    #[test]
    fn test_to_slice() {
        let mut buf = [0u8; 32];

        let used = postcard::to_slice(&true, &mut buf).expect("Failed to serialize");
        assert_eq!(used, &[0x01]);
    }
}

//! This module decribes events that cause updates to the TUI.
use std::{
    convert::Into,
    io::{self},
    sync::mpsc::Sender,
    thread::Builder,
};

use color_eyre::{
    Result,
    eyre::{WrapErr, eyre},
};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use ratatui::crossterm::event::Event;
use sc_messages::icd::{MAX_HOST_MESSAGE_SIZE, MAX_MCU_MESSAGE_SIZE, McuMessage};
use serial2::SerialPort;
use static_cell::ConstStaticCell;

/// The buffer to use when reading [`McuMessage`]s.
///
/// Meant for [`spawn_mcu_thread`].
pub static READ_BUFFER: ConstStaticCell<[u8; MAX_MCU_MESSAGE_SIZE]> = ConstStaticCell::new([0; _]);

/// All possible TUI events.
#[derive(Clone, Debug)]
pub enum TuiEvent {
    /// Crossterm events such as keyboard inputs.
    ///
    /// These events are emitted by the terminal.
    Crossterm(Event),
    /// Events from the MCU connection.
    Mcu(McuMessage),
}

impl From<McuMessage> for TuiEvent {
    fn from(value: McuMessage) -> Self {
        Self::Mcu(value)
    }
}

/// Spawns the thread that checks for crossterm events.
///
/// # Errors
/// Returns an error if the thread fails to spawn.
pub fn spawn_crossterm_thread(to_app: Sender<Result<TuiEvent>>) -> io::Result<()> {
    Builder::new()
        .name("Crossterm Events".to_string())
        .spawn(move || {
            loop {
                let event = crossterm::event::read();
                if to_app
                    .send(
                        event
                            .map(TuiEvent::Crossterm)
                            .wrap_err("Failed to read crossterm event"),
                    )
                    .is_err()
                {
                    // The receiver was dropped, so the program is ending.
                    return;
                }
            }
        })
        // We don't need the JoinHandle
        .map(|_| {})
}

/// Spawns the thread that checks for [`McuMessage`]s.
///
/// # Errors
/// Returns an error if the thread fails to spawn.
pub fn spawn_mcu_thread(
    serial: &'static SerialPort,
    buffer: &'static mut [u8; MAX_HOST_MESSAGE_SIZE],
    to_app: Sender<Result<TuiEvent>>,
) -> io::Result<()> {
    Builder::new()
        .name("MCU Messages".to_string())
        .spawn(move || {
            let mut accumulator = CobsAccumulator::<MAX_MCU_MESSAGE_SIZE>::new();
            loop {
                match serial.read(buffer).wrap_err("Failed to read serial port") {
                    Ok(num_bytes) => {
                        match accumulator.feed::<McuMessage>(&buffer[..num_bytes]) {
                            FeedResult::Consumed => {}
                            FeedResult::OverFull(_) => {
                                let _ = to_app.send(Err(eyre!(
                                    "Buffer could not hold the whole `McuMessage`"
                                )));
                                return;
                            }
                            FeedResult::DeserError(_) => {
                                let _ =
                                    to_app.send(Err(eyre!("Failed to deserialize `McuMessage`")));
                                return;
                            }
                            FeedResult::Success {
                                data: message,
                                remaining: _,
                            } => {
                                if to_app.send(Ok(message.into())).is_err() {
                                    // The receiver was dropped, so the program is ending.
                                    return;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        let _ = to_app.send(Err(err));
                        return;
                    }
                }
            }
        })
        // We don't need the JoinHandle
        .map(|_| {})
}

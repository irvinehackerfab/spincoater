//! This module decribes events that cause updates to the TUI.
use std::{
    convert::Into,
    io::{self},
    sync::mpsc::{Receiver, Sender},
    thread::{Builder, sleep},
    time::Duration,
};

use color_eyre::{
    Result,
    eyre::{WrapErr, eyre},
};
use postcard::{
    accumulator::{CobsAccumulator, FeedResult},
    to_slice_cobs,
};
use ratatui::crossterm::event::Event;
use sc_messages::icd::{HostMessage, MAX_HOST_MESSAGE_SIZE, MAX_MCU_MESSAGE_SIZE, McuMessage};
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
    /// It's time for the host to send a heartbeat to the MCU.
    Heartbeat,
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
pub fn spawn_rx_thread(
    serial: &'static SerialPort,
    buffer: &'static mut [u8; MAX_MCU_MESSAGE_SIZE],
    to_app: Sender<Result<TuiEvent>>,
) -> io::Result<()> {
    Builder::new()
        .name("Receive MCU Messages".to_string())
        .spawn(move || {
            let mut accumulator = CobsAccumulator::<MAX_MCU_MESSAGE_SIZE>::new();
            loop {
                match serial.read(buffer).wrap_err("Failed to read serial port") {
                    Ok(num_bytes) => {
                        let mut remaining = &buffer[..num_bytes];
                        while !remaining.is_empty() {
                            remaining = match accumulator.feed::<McuMessage>(remaining) {
                                FeedResult::Consumed => break,
                                FeedResult::OverFull(remaining) => {
                                    let _ = to_app.send(Err(eyre!(
                                        "Buffer could not hold the whole `McuMessage`"
                                    )));
                                    remaining
                                }
                                FeedResult::DeserError(remaining) => {
                                    let _ = to_app
                                        .send(Err(eyre!("Failed to deserialize `McuMessage`")));
                                    remaining
                                }
                                FeedResult::Success {
                                    data: message,
                                    remaining,
                                } => {
                                    if to_app.send(Ok(message.into())).is_err() {
                                        // The receiver was dropped, so the program is ending.
                                        return;
                                    }
                                    remaining
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

/// Spawns the thread that sends for [`HostMessage`]s.
///
/// # Errors
/// Returns an error if the thread fails to spawn.
pub fn spawn_tx_thread(
    serial: &'static SerialPort,
    buffer: &'static mut [u8; MAX_HOST_MESSAGE_SIZE],
    to_app: Sender<Result<TuiEvent>>,
    from_all: Receiver<HostMessage>,
) -> io::Result<()> {
    Builder::new()
        .name("Send Host Messages".to_string())
        .spawn(move || {
            loop {
                match from_all
                    .recv()
                    .wrap_err("Failed to get HostMessage to send")
                {
                    Ok(message) => {
                        match to_slice_cobs(&message, buffer)
                            .wrap_err("Failed to serialize HostMessage")
                        {
                            Ok(used) => {
                                if let Err(err) = serial
                                    .write_all(used)
                                    .wrap_err("Failed to write to serial port")
                                {
                                    let _ = to_app.send(Err(err));
                                    return;
                                }
                                // Flush the serial port.
                                // We always flush here because the ESP32's UART FIFO is only 128 bytes.
                                // We don't have flow control, so if we send too much data at once, it could overflow the buffer.
                                if let Err(err) =
                                    serial.flush().wrap_err("Failed to flush serial port")
                                {
                                    let _ = to_app.send(Err(err));
                                    return;
                                }
                            }
                            Err(err) => {
                                let _ = to_app.send(Err(err));
                                return;
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

/// Spawns the thread sends heartbeats to keep the serial port from timing out.
///
/// # Errors
/// Returns an error if the thread fails to spawn.
pub fn spawn_heartbeat_thread(to_mcu: Sender<HostMessage>) -> io::Result<()> {
    Builder::new()
        .name("Heartbeat".to_string())
        .spawn(move || {
            loop {
                if to_mcu.send(HostMessage::Heartbeat).is_err() {
                    // The receiver was dropped, so the program is ending.
                    return;
                }
                // Sleep for one second because the serial port timeout is 3 seconds.
                sleep(Duration::from_secs(1));
            }
        })
        // We don't need the JoinHandle
        .map(|_| {})
}

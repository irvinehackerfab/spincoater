//! This module contains functionality for communicating with the server over UART.

use embassy_executor::task;
use embedded_io_async::Write;
use esp_hal::{
    Async,
    gpio::Output,
    uart::{UartRx, UartTx},
};
use postcard::{
    accumulator::{CobsAccumulator, FeedResult},
    to_slice_cobs,
};
use sc_messages::{
    icd::{self, HostMessage, MAX_HOST_MESSAGE_SIZE, MAX_MCU_MESSAGE_SIZE, McuMessage},
    motion_profile, vacuum_pump,
};
use static_cell::ConstStaticCell;

use crate::{RunnerRequestSender, RunnerResponseReceiver, RunnerResponseSenderMutex};

/// The buffer for reading from the accumulator.
pub static READ_BUFFER: ConstStaticCell<[u8; MAX_HOST_MESSAGE_SIZE]> = ConstStaticCell::new([0; _]);

/// The accumulator for reading from the read buffer.
pub static READ_ACCUMULATOR: ConstStaticCell<CobsAccumulator<MAX_HOST_MESSAGE_SIZE>> =
    ConstStaticCell::new(CobsAccumulator::new());

/// The buffer for writing to UART.
pub static SEND_BUFFER: ConstStaticCell<[u8; MAX_MCU_MESSAGE_SIZE]> = ConstStaticCell::new([0; _]);

/// The half of the server that receives messages from the host PC.
pub struct ServerRx {
    from_host: UartRx<'static, Async>,
    read_buffer: &'static mut [u8; MAX_HOST_MESSAGE_SIZE],
    accumulator: &'static mut CobsAccumulator<MAX_HOST_MESSAGE_SIZE>,
    to_runner: RunnerRequestSender,
    to_tx: &'static RunnerResponseSenderMutex,
    vacuum_pump: Output<'static>,
}

impl ServerRx {
    /// Creates a new server receiver.
    pub const fn new(
        rx: UartRx<'static, Async>,
        read_buffer: &'static mut [u8; MAX_HOST_MESSAGE_SIZE],
        accumulator: &'static mut CobsAccumulator<MAX_HOST_MESSAGE_SIZE>,
        to_runner: RunnerRequestSender,
        to_tx: &'static RunnerResponseSenderMutex,
        vacuum_pump: Output<'static>,
    ) -> Self {
        Self {
            from_host: rx,
            read_buffer,
            accumulator,
            to_runner,
            to_tx,
            vacuum_pump,
        }
    }

    /// Continuously reads [`HostMessage`]s from UART.
    pub async fn read_messages(&mut self) -> ! {
        loop {
            // Read bytes from UART
            let Ok(num_bytes) = self.from_host.read_async(self.read_buffer).await
            // .inspect_err(|err| println!("Read error: {err}"))
            else {
                // The best we can do here is report the error and restart.
                Self::send_to_tx(self.to_tx, &McuMessage::Error(icd::Error::UartReadFailed)).await;
                continue;
            };
            // Save the unused bytes to a slice that can be shrunk later.
            let mut remaining = &self.read_buffer[..num_bytes];
            // println!("Buffer looks like {serialized:?}");
            while !remaining.is_empty() {
                // There are bytes left to deserialize.
                remaining = match self.accumulator.feed_ref::<HostMessage>(remaining) {
                    // All of the bytes have been used.
                    FeedResult::Consumed => break,
                    // Deserialization failed and the accumulator reset, but there still may be bytes left to use.
                    FeedResult::OverFull(remaining) | FeedResult::DeserError(remaining) => {
                        // println!("Failed to deserialize");
                        Self::send_to_tx(
                            self.to_tx,
                            &McuMessage::Error(icd::Error::DeserializationFailed),
                        )
                        .await;
                        remaining
                    }
                    // A message has been deserialized, but there still may be bytes left to use.
                    FeedResult::Success {
                        data: message,
                        remaining,
                    } => {
                        // println!("Received message: {message:#?}");
                        match message {
                            HostMessage::MotionProfile(message) => {
                                Self::send_to_runner(&mut self.to_runner, &message).await;
                            }
                            HostMessage::VacuumPump(message) => match message {
                                vacuum_pump::HostMessage::Enable => self.vacuum_pump.set_high(),
                                vacuum_pump::HostMessage::Disable => self.vacuum_pump.set_low(),
                            },
                            HostMessage::Disconnecting => {
                                Self::send_to_runner(
                                    &mut self.to_runner,
                                    &motion_profile::HostMessage::Stop,
                                )
                                .await;
                            }
                            HostMessage::Heartbeat => {
                                Self::send_to_tx(self.to_tx, &McuMessage::Heartbeat).await;
                            }
                        }
                        remaining
                    }
                }
            }
        }
    }

    /// Sends a message to the runner.
    ///
    /// This has to be an associated function so it can be called while other parts of [`ServerRx`] are mutably borrowed.
    async fn send_to_runner(
        to_runner: &mut RunnerRequestSender,
        message: &motion_profile::HostMessage,
    ) {
        let buf = to_runner.send().await;
        *buf = message.clone();
        to_runner.send_done();
    }

    /// Sends a message to the TX side of the server.
    ///
    /// This has to be an associated function so it can be called while other parts of [`ServerRx`] are mutably borrowed.
    async fn send_to_tx(to_tx: &RunnerResponseSenderMutex, message: &McuMessage) {
        let mut lock = to_tx.lock().await;
        let buf = lock.send().await;
        *buf = message.clone();
        lock.send_done();
    }
}

/// Runs [`ServerRx`].
#[task]
pub async fn run_server_rx(mut server_rx: ServerRx) {
    server_rx.read_messages().await;
}

/// The half of the server that sends messages to the host PC.
pub struct ServerTx {
    to_host: UartTx<'static, Async>,
    send_buffer: &'static mut [u8; MAX_MCU_MESSAGE_SIZE],
    from_all: RunnerResponseReceiver,
}

impl ServerTx {
    /// Creates a new server sender.
    pub const fn new(
        tx: UartTx<'static, Async>,
        send_buffer: &'static mut [u8; MAX_MCU_MESSAGE_SIZE],
        from_all: RunnerResponseReceiver,
    ) -> Self {
        Self {
            to_host: tx,
            send_buffer,
            from_all,
        }
    }

    /// Continuously sends [`McuMessage`]s to UART.
    pub async fn send_messages(&mut self) -> ! {
        loop {
            // Get message
            let message = self.from_all.receive().await;
            // Serialize
            let Ok(used) = to_slice_cobs(message, self.send_buffer)
            // .inspect_err(|err| println!("Failed to serialize: {err}"))
            else {
                continue;
            };
            // Release the message
            self.from_all.receive_done();
            // Send bytes to UART
            if self.to_host.write_all(used).await.is_err() {
                // println!("Failed to write to UART: {err}");
                continue;
            }
            // Flush UART
            let _ = self.to_host.flush_async().await;
            // println!("Failed to flush: {err}");
            // println!("Sent message");
        }
    }
}

/// Runs [`ServerTx`].
#[task]
pub async fn run_server_tx(mut server_tx: ServerTx) {
    server_tx.send_messages().await;
}

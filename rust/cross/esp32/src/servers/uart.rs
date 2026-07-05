//! This module contains functionality for communicating with the server over UART.

use embassy_executor::task;
use embassy_sync::signal::Signal;
use embedded_io::Write;
use esp_hal::{
    Blocking,
    gpio::Output,
    handler,
    uart::{UartRx, UartTx},
};
use esp_sync::RawMutex;
use postcard::{from_bytes_cobs, to_slice_cobs};
use sc_messages::{
    icd::{HostMessage, MAX_HOST_MESSAGE_SIZE, MAX_MCU_MESSAGE_SIZE},
    motion_profile, vacuum_pump,
};
use static_cell::ConstStaticCell;

use crate::{RunnerRequestSender, RunnerResponseReceiver};

/// The signal for [`ServerRx`] to start reading from UART.
static DETECTED_END: Signal<RawMutex, ()> = Signal::new();

/// The buffer for reading from UART.
pub static READ_BUFFER: ConstStaticCell<[u8; MAX_HOST_MESSAGE_SIZE]> = ConstStaticCell::new([0; _]);

/// The buffer for writing to UART.
pub static SEND_BUFFER: ConstStaticCell<[u8; MAX_MCU_MESSAGE_SIZE]> = ConstStaticCell::new([0; _]);

/// This interrupt handler signals [`ServerRx`] to read from UART whenever it is called.
#[handler]
pub fn interrupt_handler() {
    DETECTED_END.signal(());
}

/// The half of the server that receives messages from the host PC.
pub struct ServerRx {
    from_host: UartRx<'static, Blocking>,
    read_buffer: &'static mut [u8; MAX_HOST_MESSAGE_SIZE],
    to_runner: RunnerRequestSender,
    vacuum_pump: Output<'static>,
}

impl ServerRx {
    /// Creates a new server receiver.
    pub fn new(
        rx: UartRx<'static, Blocking>,
        read_buffer: &'static mut [u8; MAX_HOST_MESSAGE_SIZE],
        to_runner: RunnerRequestSender,
        vacuum_pump: Output<'static>,
    ) -> Self {
        Self {
            from_host: rx,
            read_buffer,
            to_runner,
            vacuum_pump,
        }
    }

    /// Continuously reads [`HostMessage`]s from UART.
    ///
    /// # Panics
    /// Panics if:
    /// - UART fails to read
    /// - Deserialization fails
    pub async fn read_messages(&mut self) -> ! {
        loop {
            DETECTED_END.wait().await;
            let num_bytes = self
                .from_host
                .read(self.read_buffer)
                // TODO: Maybe send the errors back to the PC
                .expect("Failed to read from UART");
            let message = from_bytes_cobs::<HostMessage>(&mut self.read_buffer[..num_bytes])
                .expect("Deserialization failed");
            match message {
                HostMessage::MotionProfile(message) => {
                    let buf = self.to_runner.send().await;
                    *buf = message;
                    self.to_runner.send_done();
                }
                HostMessage::VacuumPump(message) => match message {
                    vacuum_pump::HostMessage::Enable => self.vacuum_pump.set_high(),
                    vacuum_pump::HostMessage::Disable => self.vacuum_pump.set_low(),
                },
                HostMessage::Disconnecting => {
                    let buf = self.to_runner.send().await;
                    *buf = motion_profile::HostMessage::Stop;
                    self.to_runner.send_done();
                }
            }
        }
    }
}

/// Runs [`ServerRx`].
#[task]
pub async fn run_server_rx(mut server_rx: ServerRx) {
    server_rx.read_messages().await;
}

/// The half of the server that sends messages to the host PC.
pub struct ServerTx {
    to_host: UartTx<'static, Blocking>,
    send_buffer: &'static mut [u8; MAX_MCU_MESSAGE_SIZE],
    from_runner: RunnerResponseReceiver,
}

impl ServerTx {
    /// Creates a new server sender.
    pub fn new(
        tx: UartTx<'static, Blocking>,
        send_buffer: &'static mut [u8; MAX_MCU_MESSAGE_SIZE],
        from_runner: RunnerResponseReceiver,
    ) -> Self {
        Self {
            to_host: tx,
            send_buffer,
            from_runner,
        }
    }

    /// Continuously sends [`McuMessage`]s to UART.
    ///
    /// # Panics
    /// Panics if:
    /// - Serialization fails
    /// - UART fails to write
    pub async fn send_messages(&mut self) -> ! {
        loop {
            let message = self.from_runner.receive().await;
            let used = to_slice_cobs(message, self.send_buffer).expect("Serialization failed");
            self.from_runner.receive_done();
            self.to_host
                .write_all(used)
                .expect("Failed to write to UART");
        }
    }
}

/// Runs [`ServerTx`].
#[task]
pub async fn run_server_tx(mut server_tx: ServerTx) {
    server_tx.send_messages().await;
}

/// If this ever fails, [`ServerTx::send_messages`] may block while writing and you should consider using async UART.
const _: () = {
    assert!(MAX_MCU_MESSAGE_SIZE <= 128);
};

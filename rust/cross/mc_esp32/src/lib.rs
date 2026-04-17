//! This crate contains all ESP32-specific spincoater functionality.
//! It is meant to be compiled with Espressif's toolchain, not the regular Rust toolchain. See the README for more information.
#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![warn(clippy::large_stack_frames)]

use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex},
    signal::Signal,
    zerocopy_channel::Channel,
};
use embassy_time::Duration;
use esp_hal::system::Stack;
use sc_messages::commands::{Command, CommandRefused};
use static_cell::{ConstStaticCell, StaticCell};

pub mod gpio;
pub mod motion_profile;
pub mod rpc;
pub mod uart;

/// The static variable that holds the second core stack.
pub static SECOND_CORE_STACK: ConstStaticCell<Stack<2048>> = ConstStaticCell::new(Stack::new());

/// The period that the main control loop runs at.
///
/// The fastest it can run is about 3 milliseconds.
pub const LOOP_PERIOD: Duration = Duration::from_millis(20);

/// The buffer used by [`COMMAND_CHANNEL`].
///
/// The buffer's starting values are irrelevant.
pub static COMMAND_CHANNEL_BUFFER: ConstStaticCell<[Command; 4]> =
    ConstStaticCell::new([Command::Stop, Command::Stop, Command::Stop, Command::Stop]);

/// Used for passing commands from the server to the command handler.
///
/// This uses [`NoopRawMutex`] because data is only shared in one executor.
///
/// This uses a zerocopy channel because [`Command`]s are expensive to copy.
pub static COMMAND_CHANNEL: StaticCell<Channel<NoopRawMutex, Command>> = StaticCell::new();

/// Used for passing command responses from the command handler to the server.
///
/// This uses [`CriticalSectionRawMutex`] because the signal must be [`Sync`] because
/// the static is shared between cores.
///
/// This is a signal because the server always waits for one response after sending a command.
pub static COMMAND_RESPONSE_SIGNAL: Signal<CriticalSectionRawMutex, Result<(), CommandRefused>> =
    Signal::new();

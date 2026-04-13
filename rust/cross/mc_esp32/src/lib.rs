//! This crate contains all ESP32-specific spincoater functionality.
//! It is meant to be compiled with Espressif's toolchain, not the regular Rust toolchain. See the README for more information.
#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![warn(clippy::large_stack_frames)]

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use embassy_time::Duration;
use esp_hal::system::Stack;
use sc_messages::{commands::Command, motion_profile};
use static_cell::ConstStaticCell;
pub mod gpio;
pub mod uart;

/// The static variable that holds the second core stack.
pub static SECOND_CORE_STACK: ConstStaticCell<Stack<2048>> = ConstStaticCell::new(Stack::new());

/// The period that the main control loop runs at.
///
/// The fastest it can run is about 3 milliseconds.
pub const LOOP_PERIOD: Duration = Duration::from_millis(20);

/// The maximum number of messages allowed at a time in each channel to/from the message handler.
pub const HANDLER_CHANNEL_SIZE: usize = 2;
/// Used for passing commands from the socket to the command handler.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
pub static RECV_CMD_CHANNEL: ConstStaticCell<Channel<NoopRawMutex, Command, HANDLER_CHANNEL_SIZE>> =
    ConstStaticCell::new(Channel::new());

/// Used for passing info to the socket.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
pub static SEND_INFO_CHANNEL: ConstStaticCell<
    Channel<NoopRawMutex, motion_profile::State, HANDLER_CHANNEL_SIZE>,
> = ConstStaticCell::new(Channel::new());

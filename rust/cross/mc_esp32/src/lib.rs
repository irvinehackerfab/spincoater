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
    channel::Channel,
    signal::Signal,
};
use embassy_time::Duration;
use esp_hal::system::Stack;
use sc_messages::motion_profile::{Request, RequestRefused};
use static_cell::ConstStaticCell;

use crate::gpio::pwm::SETPOINT_LIST_LENGTH;

pub mod gpio;
pub mod motion_profile;
pub mod rpc;

/// The static variable that holds the second core stack.
pub static SECOND_CORE_STACK: ConstStaticCell<Stack<2048>> = ConstStaticCell::new(Stack::new());

/// The period that the main control loop runs at.
///
/// The fastest it can run is about 3 milliseconds.
pub const LOOP_PERIOD: Duration = Duration::from_millis(20);

/// The length of the buffer used by [`REQUEST_CHANNEL`].
pub const REQUEST_CHANNEL_LENGTH: usize = SETPOINT_LIST_LENGTH;

/// Used for passing motion profile requests from the server to the request handler.
///
/// This uses [`NoopRawMutex`] because data is only shared in one executor.
pub static REQUEST_CHANNEL: ConstStaticCell<
    Channel<NoopRawMutex, Request, REQUEST_CHANNEL_LENGTH>,
> = ConstStaticCell::new(Channel::new());

/// Used for passing request responses from the request handler to the server.
///
/// This uses [`CriticalSectionRawMutex`] because the signal must be [`Sync`] because
/// the static is shared between cores.
///
/// This is a signal because the server always waits for one response after sending a request.
pub static REQUEST_RESPONSE_SIGNAL: Signal<CriticalSectionRawMutex, Result<(), RequestRefused>> =
    Signal::new();

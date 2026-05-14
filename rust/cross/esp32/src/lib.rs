//! This crate contains all ESP32-specific spincoater functionality.
//! It is meant to be compiled with Espressif's toolchain, not the regular Rust toolchain. See the README for more information.
#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![warn(clippy::large_stack_frames)]

pub mod gpio;
pub mod pid;
pub mod rpc;
pub mod runners;

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel, signal::Signal};
use embassy_time::Duration;
use esp_hal::system::Stack;
use esp_rtos::embassy::InterruptExecutor;
use sc_messages::motion_profile::{Request, RequestRefused};
use static_cell::{ConstStaticCell, StaticCell};

use crate::gpio::pwm::SETPOINT_LIST_LENGTH;

/// The stack of the second core.
pub static SECOND_CORE_STACK: ConstStaticCell<Stack<8192>> = ConstStaticCell::new(Stack::new());

/// The executor for the second core.
pub static SECOND_CORE_EXECUTOR: StaticCell<InterruptExecutor<2>> = StaticCell::new();

/// The period that the main control loop runs at.
///
/// The further you raise this past `20`, the greater your risk of filling up [`gpio::encoder::RPM_RING_BUFFER`] is.
/// The only consequence of this is a less accurate moving average.
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
/// This is a signal because the server always waits for one response after sending a request.
pub static REQUEST_RESPONSE_SIGNAL: ConstStaticCell<
    Signal<NoopRawMutex, Result<(), RequestRefused>>,
> = ConstStaticCell::new(Signal::new());

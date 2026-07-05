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
pub mod runners;
pub mod servers;

use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    zerocopy_channel::{Channel, Receiver, Sender},
};
use embassy_time::Duration;
use esp_hal::system::Stack;
use esp_rtos::embassy::InterruptExecutor;
use sc_messages::motion_profile;
use static_cell::{ConstStaticCell, StaticCell};

/// The stack of the second core.
pub static SECOND_CORE_STACK: ConstStaticCell<Stack<8192>> = ConstStaticCell::new(Stack::new());

/// The executor for the second core.
pub static SECOND_CORE_EXECUTOR: StaticCell<InterruptExecutor<2>> = StaticCell::new();

/// The period that the main control loop runs at.
///
/// The further you raise this past `20`, the greater your risk of filling up [`gpio::encoder::RPM_RING_BUFFER`] is.
/// The only consequence of this is a less accurate moving average.
pub const LOOP_PERIOD: Duration = Duration::from_millis(20);

use crate::gpio::pwm::SETPOINT_LIST_LENGTH;

/// The buffer used by [`RUNNER_REQUEST_CHANNEL`].
pub static RUNNER_REQUEST_BUFFER: ConstStaticCell<
    [motion_profile::HostMessage; SETPOINT_LIST_LENGTH],
> = ConstStaticCell::new([const { motion_profile::HostMessage::Stop }; _]);

/// Used for passing [`HostMessage`]s from the server.
///
/// This is zerocopy because the messages are expensive to copy.
/// This uses [`NoopRawMutex`] because data is only shared in one executor.
pub static RUNNER_REQUEST_CHANNEL: StaticCell<Channel<NoopRawMutex, motion_profile::HostMessage>> =
    StaticCell::new();

pub type RunnerRequestReceiver = Receiver<'static, NoopRawMutex, motion_profile::HostMessage>;
pub type RunnerRequestSender = Sender<'static, NoopRawMutex, motion_profile::HostMessage>;

/// The buffer used by [`RUNNER_RESPONSE_CHANNEL`].
pub static RUNNER_RESPONSE_BUFFER: ConstStaticCell<[motion_profile::McuMessage; 4]> =
    ConstStaticCell::new([const { motion_profile::McuMessage::Finished }; _]);

/// Used for passing [`McuMessage`]s to the server.
///
/// This uses [`NoopRawMutex`] because data is only shared in one executor.
pub static RUNNER_RESPONSE_CHANNEL: StaticCell<Channel<NoopRawMutex, motion_profile::McuMessage>> =
    StaticCell::new();

pub type RunnerResponseReceiver = Receiver<'static, NoopRawMutex, motion_profile::McuMessage>;
pub type RunnerResponseSender = Sender<'static, NoopRawMutex, motion_profile::McuMessage>;

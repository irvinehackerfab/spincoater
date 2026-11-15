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

use esp_hal::system::Stack;
use esp_rtos::embassy::InterruptExecutor;
use static_cell::{ConstStaticCell, StaticCell};

/// The stack of the second core.
pub static SECOND_CORE_STACK: ConstStaticCell<Stack<8192>> = ConstStaticCell::new(Stack::new());

/// The executor for the second core.
pub static SECOND_CORE_EXECUTOR: StaticCell<InterruptExecutor<2>> = StaticCell::new();

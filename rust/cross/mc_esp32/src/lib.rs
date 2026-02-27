//! This crate contains all ESP32-specific spincoater functionality.
//! It is meant to be compiled with Espressif's toolchain, not the regular Rust toolchain. See the README for more information.
#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::system::Stack;
use static_cell::StaticCell;
pub mod gpio;
pub mod wifi;

/// The static variable that holds the second core stack.
pub static SECOND_CORE_STACK: StaticCell<Stack<1024>> = StaticCell::new();

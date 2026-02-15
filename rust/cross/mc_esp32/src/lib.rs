#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::system::Stack;
use static_cell::StaticCell;
pub mod encoder;
pub mod pwm;
pub mod wifi;

pub static SECOND_CORE_STACK: StaticCell<Stack<8192>> = StaticCell::new();

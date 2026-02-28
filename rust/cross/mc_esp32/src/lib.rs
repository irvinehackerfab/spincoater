//! This crate contains all ESP32-specific spincoater functionality.
//! It is meant to be compiled with Espressif's toolchain, not the regular Rust toolchain. See the README for more information.
#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![warn(clippy::large_stack_frames)]

use core::fmt::Debug;

use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{self, TrySendError},
};
use esp_hal::system::Stack;
use esp_println::println;
use static_cell::StaticCell;
pub mod gpio;
pub mod wifi;

/// The static variable that holds the second core stack.
pub static SECOND_CORE_STACK: StaticCell<Stack<1024>> = StaticCell::new();

// Todo: Once you stop getting these errors and have finalized your capacities,
// remove this function to save stack space.
/// Tries to send a message on a channel.
/// If sending fails, prints out a "buffer full" notice and calls send asynchronously.
pub async fn send_or_report_and_send<T, const N: usize>(
    sender: &channel::Sender<'_, NoopRawMutex, T, N>,
    message: T,
) where
    T: Debug,
{
    if let Err(TrySendError::Full(message)) = sender.try_send(message) {
        println!("No space for {message:?}.");
        sender.send(message).await;
    }
}

//! This module contains all encoder functionality.
//!
//! If you're looking for the interrupt service routine that handles hall effect sensor readings,
//! it's located in the [gpio](`crate::gpio`) module.
use core::{
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};

use critical_section::Mutex;
use embassy_executor::task;
use esp_hal::gpio::Input;

/// Provides the interrupt handler access to the hall effect sensor.
pub static ENCODER: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

/// The counter for the motor revolutions. This counter is equal to motor revolutions * 2.
pub static MOTOR_REVOLUTIONS_DOUBLED: AtomicU32 = AtomicU32::new(0);

#[task]
pub async fn handle_encoder(mut pin: Input<'static>) {
    loop {
        pin.wait_for_rising_edge().await;
        MOTOR_REVOLUTIONS_DOUBLED.fetch_add(1, Ordering::Relaxed);
    }
}

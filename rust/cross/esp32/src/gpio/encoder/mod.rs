//! This module contains all encoder functionality.
//!
//! If you're looking for the interrupt service routine that handles hall effect sensor readings,
//! it's located in the [gpio](`crate::gpio`) module.
use core::sync::atomic::{AtomicU32, Ordering};
use embassy_executor::task;
use esp_hal::gpio::Input;
use esp_sync::NonReentrantMutex;

/// Provides the interrupt handler access to the hall effect sensor.
pub static ENCODER: NonReentrantMutex<Option<Input>> = NonReentrantMutex::new(None);

/// The counter for the motor revolutions. This counter is equal to motor revolutions * 2.
pub static MOTOR_REVOLUTIONS_DOUBLED: AtomicU32 = AtomicU32::new(0);

#[task]
pub async fn handle_encoder(mut pin: Input<'static>) {
    loop {
        pin.wait_for_rising_edge().await;
        MOTOR_REVOLUTIONS_DOUBLED.fetch_add(1, Ordering::Relaxed);
    }
}

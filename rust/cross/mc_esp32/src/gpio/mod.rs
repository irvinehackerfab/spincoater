//! This module contains all GPIO functionality.
//!
//! See [Espressif's documentation](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/peripherals/gpio.html)
//! for more information on GPIO.

use core::sync::atomic::Ordering;

use esp_hal::handler;

use crate::gpio::encoder::{ENCODER, MOTOR_REVOLUTIONS_DOUBLED};

pub mod display;
pub mod encoder;
pub mod pwm;

/// The handler for all GPIO interrupts.
/// Since you can only have one GPIO handler,
/// you must perform pin-specific code by checking the interrupt status of each pin.
///
/// # Panics
/// Panics if [`MOTOR_REVOLUTIONS_DOUBLED`] overflows.
#[handler]
pub fn interrupt_handler() {
    critical_section::with(|cs| {
        let mut encoder = ENCODER.borrow_ref_mut(cs);
        let Some(encoder) = encoder.as_mut() else {
            return;
        };
        if encoder.is_interrupt_set() {
            MOTOR_REVOLUTIONS_DOUBLED.fetch_add(1, Ordering::Relaxed);
            encoder.clear_interrupt();
        }
    });
}

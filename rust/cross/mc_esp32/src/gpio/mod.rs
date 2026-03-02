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
    let encoder_rising_edge = critical_section::with(|cs| {
        let encoder = ENCODER.borrow_ref(cs);
        match encoder.as_ref() {
            Some(encoder) => encoder.is_interrupt_set(),
            // Some other interrupt occurred before the encoder was set up.
            None => false,
        }
    });
    if encoder_rising_edge {
        let previous_value = MOTOR_REVOLUTIONS_DOUBLED.fetch_add(1, Ordering::Relaxed);
        // If the previous value was the highest possible value, the counter overflowed.
        assert_ne!(previous_value, u32::MAX);
    }
}

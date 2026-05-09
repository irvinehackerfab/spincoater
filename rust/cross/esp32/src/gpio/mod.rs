//! This module contains all GPIO functionality.
//!
//! See [Espressif's documentation](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/peripherals/gpio.html)
//! for more information on GPIO.

use esp_hal::handler;

use crate::gpio::encoder::{ENCODER, ENCODER_STATE, EncoderState};

pub mod display;
pub mod encoder;
pub mod pwm;

/// The handler for all GPIO interrupts.
/// Since you can only have one GPIO handler,
/// you must perform pin-specific code by checking the interrupt status of each pin.
///
/// See [`set_interrupt_handler`](esp_hal::gpio::Io::set_interrupt_handler) for ISR requirements,
/// and see [`listen`](esp_hal::gpio::Input::listen) for an example.
///
/// # Panics
/// Panics if [`MOTOR_REVOLUTIONS_DOUBLED`] overflows.
#[handler]
pub fn interrupt_handler() {
    if ENCODER.with(|encoder| {
        let Some(encoder) = encoder.as_mut() else {
            // A GPIO interrupt fired before the encoder was initialized.
            return false;
        };
        if encoder.is_interrupt_set() {
            // This must be called to ensure that the interrupt handler can be reliably reused in the future.
            encoder.clear_interrupt();
            return true;
        }
        false
    }) {
        // An interrupt occurred, so it's time to calculate the rpm.
        ENCODER_STATE.with(EncoderState::calculate_rpm);
    }
}

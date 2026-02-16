use esp_hal::handler;

use crate::gpio::encoder::ENCODER;

pub mod encoder;
pub mod pwm;

/// The handler for all GPIO interrupts.
/// Since you can only have one handler,
/// you must perform pin-specific code by checking the interrupt status of each pin.
#[handler]
pub fn interrupt_handler() {
    let encoder_rising_edge = critical_section::with(|cs| {
        let mut encoder = ENCODER.borrow_ref_mut(cs);
        let Some(encoder) = encoder.as_mut() else {
            // Some other interrupt has occurred before the encoder was set up.
            return false;
        };
        encoder.is_interrupt_set()
    });
    if encoder_rising_edge {
        todo!("Increment counter");
    }
}

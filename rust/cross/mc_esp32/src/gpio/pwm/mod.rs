//! This module contains the PWM output constants used by all ESP32 programs.
//!
use esp_hal::time::Rate;

// PWM output constants
/// The current motor controller reads PWM at 50 Hz.
pub const FREQUENCY: Rate = Rate::from_hz(50);

/// This prescaler is what lowers the peripheral clock frequency down to a level that is usable by the timer.
///
/// The timer has its own prescaler, which it can determine automatically as long as the equation
///
/// `timer_prescaler` = `160_000_000` / ([`PERIPHERAL_CLOCK_PRESCALER`] + 1) / ([`PERIOD`] + 1) / [`FREQUENCY`] - 1
///
/// results in a value in the range 0..[`u8::MAX`].
///
/// Therefore, this should be set to the lowest value where `timer_prescaler` is still within 0..255.
///
/// See [the Wikipedia page](https://en.wikipedia.org/wiki/Prescaler) on prescalers for more info.
pub const PERIPHERAL_CLOCK_PRESCALER: u8 = 0;

/// The value corresponding to 100% PWM period - 1.
/// We can configure this to whatever we like.
///
/// Since we can only control the duty cycle with whole numbers,
/// setting it to the highest allowed value gives us the best control over the output.
///
/// However, if the equation
///
/// `timer_prescaler` = `160_000_000` / ([`PERIPHERAL_CLOCK_PRESCALER`] + 1) / ([`PERIOD`] + 1) / [`FREQUENCY`] - 1
///
/// results in a decimal value, [`esp_hal`](esp_hal::mcpwm::PeripheralClockConfig::timer_clock_with_frequency) will round it,
/// resulting in a loss of PWM output accuracy.
///
/// This is currently set to the highest possible value that also results in a whole-numbered `timer_prescaler`.
pub const PERIOD: u16 = sc_messages::PERIOD - 1;

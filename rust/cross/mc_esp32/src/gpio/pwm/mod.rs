//! This module contains PWM output functionality.
//!
//! See [this Desmos graph](https://www.desmos.com/calculator/rtwkr7v4ko) for more information about PWM conversions.
use esp_hal::time::Rate;
use heapless::Vec;
use muldiv::MulDiv;
use sc_messages::{
    motion_profile::{MAX_SETPOINTS, Setpoint},
    pwm::{DutyCycle, MAX_POWER_DUTY},
};
use static_cell::StaticCell;

/// The current motor controller reads PWM at 490 Hz.
pub const FREQUENCY: Rate = Rate::from_hz(490);

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

/// The value corresponding to __100% PWM period - 1.__
/// (100% PWM period is [`sc_messages::PERIOD`].)
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
/// This is currently set to the highest possible value that results in a `timer_prescaler` that is as close to a whole number as possible.
/// Right now, `timer_prescaler` is 4 (rounded down from 4.00000937502).
pub const PERIOD: u16 = sc_messages::pwm::PERIOD - 1;

pub const SETPOINT_LIST_LENGTH: usize = MAX_SETPOINTS + 1;

/// The static cell for storing a motion profile.
pub static SETPOINTS: StaticCell<Vec<Setpoint, SETPOINT_LIST_LENGTH>> = StaticCell::new();

/// The slope numerator for the conversion from plate RPM to internal pulse width.
pub const CONVERSION_NUMERATOR: u16 = 991;

/// The slope denominator for the conversion from plate RPM to internal pulse width.
pub const CONVERSION_DENOMINATOR: u16 = 100;

/// The intercept for the conversion from plate RPM to internal pulse width.
pub const CONVERSION_INTERCEPT: u16 = 33_920;

#[must_use]
pub fn plate_rpm_to_pulse_width(rpm: u16) -> DutyCycle {
    match rpm.mul_div_round(CONVERSION_NUMERATOR, CONVERSION_DENOMINATOR) {
        Some(value) => value.saturating_add(CONVERSION_INTERCEPT).into(),
        None => MAX_POWER_DUTY,
    }
}

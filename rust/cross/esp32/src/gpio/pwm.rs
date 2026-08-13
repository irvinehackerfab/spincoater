//! This module contains PWM output functionality.

use core::ops::{Div, Mul};

use esp_hal::time::Rate;
use heapless::Vec;
use sc_messages::{
    motion_profile::{MAX_SETPOINTS, Setpoint},
    pwm::DutyCycle,
};
use static_cell::ConstStaticCell;

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
/// This is currently set to the highest possible value that also results in a whole-numbered `timer_prescaler`.
pub const PERIOD: u16 = sc_messages::pwm::PERIOD - 1;

pub const SETPOINT_LIST_LENGTH: usize = MAX_SETPOINTS + 1;

/// The static cell for storing a motion profile.
pub static SETPOINTS: ConstStaticCell<Vec<Setpoint, SETPOINT_LIST_LENGTH>> =
    ConstStaticCell::new(Vec::from_array([Setpoint { rpm: 0, time: 0 }]));

/// Uses a linear equation between motor RPM and duty cycle to find the setpoint duty cycle.
#[must_use]
pub fn linear_conversion(setpoint_rpm: u16) -> DutyCycle {
    // These values were obtained from the `linear_regression` program.
    const RPM_TO_DUTY_NUMERATOR: u32 = 1579;
    const RPM_TO_DUTY_DENOMINATOR: u32 = 59_230;
    /// The linear relationship between motor RPM and PWM units has an intercept because the duty cycle representing 0 is nonzero.
    const RPM_TO_DUTY_INTERCEPT: u32 = 4928;

    // Everything here is in u32 to prevent overflow.
    let setpoint_rpm = u32::from(setpoint_rpm);
    // Ths arithmetic here is saturating because it will never exceed u32::MAX.
    let duty = setpoint_rpm
        .mul(RPM_TO_DUTY_NUMERATOR)
        .div(RPM_TO_DUTY_DENOMINATOR)
        .saturating_add(RPM_TO_DUTY_INTERCEPT);
    duty.into()
}

/// Uses a cubic equation between motor RPM and duty cycle to find the setpoint duty cycle.
#[must_use]
pub fn cubic_conversion(setpoint_rpm: u16) -> DutyCycle {
    // We have to use floating point arithmetic because `CUBIC_D` and `CUBIC_C` are too small to turn into numerators and denominators.
    // These values were obtained from inputting a long motor log into a [model fitting website](https://livephysics.com/labs/scientific-data-graphing-lab/).
    /// The coefficient of the x^3 term in the duty cycle vs motor RPM cubic equation.
    const CUBIC_D: f32 = 5.829e-11;
    /// The coefficient of the x^2 term in the duty cycle vs motor RPM cubic equation.
    const CUBIC_C: f32 = -9.048e-7;
    /// The coefficient of the x term in the duty cycle vs motor RPM cubic equation.
    const CUBIC_B: f32 = 0.0179;
    /// The intercept in the duty cycle vs motor RPM cubic equation.
    const CUBIC_A: f32 = 5_005.072;

    let setpoint_rpm = f32::from(setpoint_rpm);
    let setpoint_rpm_squared = setpoint_rpm * setpoint_rpm;
    let setpoint_rpm_cubed = setpoint_rpm * setpoint_rpm * setpoint_rpm;
    let duty = setpoint_rpm_cubed * CUBIC_D
        + setpoint_rpm_squared * CUBIC_C
        + setpoint_rpm * CUBIC_B
        + CUBIC_A;
    // Rounding code inspired by [micromath](https://docs.rs/micromath/latest/src/micromath/float/round.rs.html#7-9)
    #[allow(
        clippy::cast_possible_truncation,
        reason = "We are truncating on purpose. Even if `setpoint_rpm` was `u16::MAX`, `duty` would still be less than `u16::MAX`."
    )]
    #[allow(clippy::cast_sign_loss, reason = "`duty` is guaranteed to be positive")]
    DutyCycle::from((duty + 0.5) as u16)
}

//! This module contains PWM output functionality.
use esp_hal::time::Rate;
use heapless::Vec;
use sc_messages::motion_profile::{MAX_SETPOINTS, Setpoint};
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

/// Since the relationship betwen motor RPM and PWM units is mostly linear, we can just use a conversion factor.
/// This value was obtained from the `linear_regression` program.
pub const RPM_TO_DUTY_NUMERATOR: u32 = 31_309;

/// Since the relationship betwen motor RPM and PWM units is mostly linear, we can just use a conversion factor.
/// This value was obtained from the `linear_regression` program.
pub const RPM_TO_DUTY_DENOMINATOR: u32 = 2_000_000;

/// The linear relationship between motor RPM and PWM units has an intercept because the duty cycle representing 0 is nonzero.
/// This value was obtained from the `linear_regression` program.
pub const RPM_TO_DUTY_INTERCEPT: u32 = 5_018;

/// This duty cycle is guaranteed to make the motor start spinning.
pub const STATIC_DUTY: u16 = 5_065;

/// The inverse of the proportional gain.
///
/// The non-inverse of `K_P` is in units of duty cycle per motor RPM error.
pub const K_P_INVERSE: i16 = 16;

//! This module contains PWM output functionality.
use esp_hal::time::Rate;
use heapless::Vec;
use sc_messages::motion_profile::{MAX_SETPOINTS, Setpoint};
use static_cell::StaticCell;

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
pub static SETPOINTS: StaticCell<Vec<Setpoint, SETPOINT_LIST_LENGTH>> = StaticCell::new();

/// The number of datapoints for the throttle curve.
pub const THROTTLE_POINTS: usize = 9;

/// The table of values we got from reading the motor RPM at multiple PWM duty cycles.
///
/// No two RPM values should be the same.
///
/// Units of `[0]`: Motor RPM (RPM)
///
/// Units of `[1]`: PWM units (microseconds * [`sc_messages::PERIOD`] * [`FREQUENCY`] seconds^-1 / 10^6 microseconds)
///
/// See [the graph](https://www.desmos.com/calculator/dtaaxpy72o) for more info.
pub const THROTTLE_CURVE: [[u32; THROTTLE_POINTS]; 2] = [
    [
        0, 9_800, 17_900, 24_900, 29_500, 32_500, 34_500, 38_100, 38_500,
    ],
    [
        4_800, 5_056, 5_120, 5_200, 5_280, 5_360, 5_440, 5_760, 6_080,
    ],
];

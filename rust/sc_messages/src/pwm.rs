//! See [this Desmos page](https://www.desmos.com/calculator/rtwkr7v4ko) for more information on pulse width conversions.
#[cfg(feature = "std")]
extern crate std;

use core::{
    fmt::{self, Display, Formatter},
    ops::Deref,
};

use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

/// The value corresponding to 100% of the PWM period.
/// See [`../cross/mc_esp32/src/pwm/mod.rs`] for an explanation on the choice for this value.
pub const PERIOD: u16 = u16::MAX - 229;

/// The current motor controller reads `1_860` microseconds (`59_520` internal PWM units) as max speed (9558 motor RPM).
pub const MAX_POWER_DUTY: DutyCycle = DutyCycle(59_520);

/// The current motor controller reads `1_060` microseconds (`33_920` internal PWM units) as 0 RPM, or brake.
pub const BRAKE_DUTY: DutyCycle = DutyCycle(33_920);

/// The current motor controller reads anything less than `1_060` microseconds (`33_920` internal PWM units) as neutral/coast.
pub const STOP_DUTY: DutyCycle = DutyCycle(25_600);

/// A duty cycle.
/// 0-100% is encoded as 0..[`PERIOD`].
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Schema)]
pub struct DutyCycle(u16);

impl Deref for DutyCycle {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u16> for DutyCycle {
    /// Wraps a [`u16`] in [`DutyCycle`].
    ///
    /// Truncates to [`MAX_POWER_DUTY`] if the value is greater than that.
    fn from(value: u16) -> Self {
        Self(value.min(*MAX_POWER_DUTY))
    }
}

impl Display for DutyCycle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The [`u16`]'s value was too high to be considered a [`DutyCycle`].
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub struct OutOfRange(pub u16);

impl Display for OutOfRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} is greater than the maximum duty cycle of {}.",
            self.0, PERIOD
        )
    }
}

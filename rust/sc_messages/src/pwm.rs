#[cfg(feature = "std")]
extern crate std;

use core::{
    fmt::{self, Display, Formatter},
    ops::Deref,
};

use serde::{Deserialize, Serialize};

/// The value corresponding to 100% of the PWM period.
/// See [`../cross/esp32/src/pwm/mod.rs`] for an explanation on the choice for this value.
pub const PERIOD: u16 = u16::MAX - 1_535;

/// The current motor controller reads 10% of [`PERIOD`] as 100% power.
pub const MAX_POWER_DUTY: u16 = PERIOD / 10;

/// The current motor controller reads 8.75% of [`PERIOD`] as 50% power.
pub const HALF_POWER_DUTY: u16 = PERIOD / 80 * 7;

/// The current motor controller reads 7.5% of [`PERIOD`] as 0% power.
///
/// 0% power means neutral.
pub const STOP_DUTY: u16 = PERIOD / 40 * 3;

/// A duty cycle.
/// 0-100% is encoded as 0..[`PERIOD`].
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
    /// Clamps `value` to a maximum of [`MAX_POWER_DUTY`].
    fn from(value: u16) -> Self {
        Self(value.min(MAX_POWER_DUTY))
    }
}

impl From<u32> for DutyCycle {
    /// Wraps a [`u32`] in [`DutyCycle`].
    ///
    /// Clamps `value` to a maximum of [`MAX_POWER_DUTY`].
    fn from(value: u32) -> Self {
        Self(value.try_into().unwrap_or(u16::MAX).min(MAX_POWER_DUTY))
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

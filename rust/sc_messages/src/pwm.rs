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
    /// Clamps `value` to a minimum of [`STOP_DUTY`] and a maximum of [`MAX_POWER_DUTY`].
    fn from(value: u16) -> Self {
        Self(value.clamp(STOP_DUTY, MAX_POWER_DUTY))
    }
}

impl From<u32> for DutyCycle {
    /// Wraps a [`u32`] in [`DutyCycle`].
    ///
    /// Clamps `value` to a minimum of [`STOP_DUTY`] and a maximum of [`MAX_POWER_DUTY`].
    fn from(value: u32) -> Self {
        #[allow(clippy::cast_possible_truncation, reason = "We just clamped the value")]
        Self(value.clamp(u32::from(STOP_DUTY), u32::from(MAX_POWER_DUTY)) as u16)
    }
}

impl Display for DutyCycle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

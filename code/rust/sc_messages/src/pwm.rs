use core::{
    fmt::{self, Display, Formatter},
    ops::{Add, Deref},
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

impl From<f32> for DutyCycle {
    /// Wraps an [`f32`] in [`DutyCycle`].
    ///
    /// Clamps `value` to a minimum of [`STOP_DUTY`] and a maximum of [`MAX_POWER_DUTY`].
    fn from(value: f32) -> Self {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        const {
            // Proves that truncation will not occur when casting.
            assert!(f32::MAX as u16 == u16::MAX);
            assert!((f32::MAX - 1.) as u16 == u16::MAX);
            // Proves that float to int casts are saturating.
            assert!(-1. as u16 == 0);
            assert!(-2. as u16 == 0);
        }

        // Rounding code inspired by [micromath](https://docs.rs/micromath/2.1.0/src/micromath/float/round.rs.html#7-9)
        // We aren't using libm or micromath here
        // because we don't need rounding with ties even or rounding with negative number support.
        // Todo: Use f32::round_ties_even once stabilized (https://doc.rust-lang.org/stable/core/f32/math/fn.round_ties_even.html)
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        Self((value.add(0.5) as u16).clamp(STOP_DUTY, MAX_POWER_DUTY))
    }
}

impl Display for DutyCycle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

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
pub const PERIOD: u16 = u16::MAX - 1_535;

/// The current motor controller reads 10% of [`PERIOD`] as 100% power.
pub const MAX_POWER_DUTY: DutyCycle = DutyCycle(PERIOD / 10);

/// The current motor controller reads 7.5% of [`PERIOD`] as 0% power.
///
/// 0% power means neutral.
pub const STOP_DUTY: DutyCycle = DutyCycle(PERIOD / 40 * 3);

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

impl TryFrom<u16> for DutyCycle {
    type Error = OutOfRange;

    /// Attempt to wrap a [`u16`] in [`DutyCycle`].
    ///
    /// This fails if the value is greater than [`PERIOD`].
    fn try_from(value: u16) -> Result<DutyCycle, OutOfRange> {
        if value <= PERIOD {
            Ok(Self(value))
        } else {
            Err(OutOfRange(value))
        }
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

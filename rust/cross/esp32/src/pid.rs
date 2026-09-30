//! This module contains our PID control code.
//!
//! Keep in mind that 1 duty cycle is approximately equal to 34 plate RPM.

use core::ops::Add;

use crate::LOOP_PERIOD_MILLIS_F32;

/// The proportional gain.
///
/// Units: duty cycle per motor RPM error.
pub const K_P: f32 = 0.125;

/// The integral gain.
///
/// Units: duty cycle per motor RPM error seconds.
// The period is constant so we can include it in `K_I`
// rather than multiplying the error by the period every time.
pub const K_I: f32 = 1. * LOOP_PERIOD_MILLIS_F32 / 1000.;

/// The integral output limit.
///
/// Units: duty cycle.
pub const I_LIMIT: f32 = 10.;

/// Calculates the difference between the setpoint and current RPM,
/// which is the negative of the RPM error.
///
/// The result is clamped to [`i16::MIN`] and [`i16::MAX`].
#[must_use]
pub fn neg_error(setpoint_rpm: u16, current_rpm: u16) -> i16 {
    setpoint_rpm
        .checked_signed_diff(current_rpm)
        .unwrap_or(if setpoint_rpm < current_rpm {
            i16::MIN
        } else {
            i16::MAX
        })
}

/// A PI controller.
///
/// You should call `next_control_output` at a regular interval.
#[derive(Debug)]
pub struct Pi {
    /// The accumulated I output.
    ///
    /// Units: duty cycle.
    i_sum: f32,
}

impl Pi {
    /// Creates a new PI controller.
    #[must_use]
    pub const fn new() -> Self {
        Self { i_sum: 0. }
    }

    /// Calculates the output.
    #[must_use]
    pub fn next_control_output(&mut self, negative_error: i16) -> f32 {
        let negative_error = f32::from(negative_error);
        let p_term = negative_error * K_P;
        self.i_sum = self
            .i_sum
            .add(negative_error * K_I)
            .clamp(-I_LIMIT, I_LIMIT);
        p_term + self.i_sum
    }

    /// Resets the I output.
    pub const fn reset_integral_output(&mut self) {
        self.i_sum = 0.;
    }
}

impl Default for Pi {
    fn default() -> Self {
        Self::new()
    }
}

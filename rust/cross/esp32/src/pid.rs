//! This module contains our PID control code.
//!
//! Keep in mind that 1 duty cycle is approximately equal to 34 plate RPM.
//!
//! It is recommended to use the [Ziegler-Nichols method](https://en.wikipedia.org/wiki/Ziegler%E2%80%93Nichols_method)
//! as described in [this video](https://youtu.be/UOuRx9Ujsog?t=543)
//! to begin tuning your PID gains.

use crate::LOOP_PERIOD_MILLIS_U64;

/// The loop period in seconds.
#[allow(clippy::cast_precision_loss, reason = "See proof below")]
const LOOP_PERIOD_SECS: f32 = LOOP_PERIOD_MILLIS_U64 as f32 / 1000.;
/// Proves that [`LOOP_PERIOD_MILLIS_U64`] fits in an [`f32`].
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
const _: () = assert!(LOOP_PERIOD_MILLIS_U64 as f32 as u64 == LOOP_PERIOD_MILLIS_U64);

/// The critical/ultimate gain, AKA the [`K_P`] at which a stable and consistent oscillation occurs.
///
/// Units: duty cycle per motor RPM error.
// This was found at a low RPM with feedforward enabled.
const K_C: f32 = 0.3;

/// The period of oscillation when [`K_C`] is used.
///
/// Units: seconds.
const P_C: f32 = 0.82;

/// The proportional gain.
///
/// Units: duty cycle per motor RPM error.
// We use 0.5 instead of 0.6 to reduce overshoot.
const K_P: f32 = 0.5 * K_C;

/// The integral gain.
///
/// Units: duty cycle per motor RPM error seconds.
const K_I: f32 = 2. * K_P / P_C;

/// The derivative gain.
///
/// Units: duty cycle per (motor RPM error per second).
const K_D: f32 = 0.125 * K_P * P_C;

/// The integral output limit.
///
/// Units: duty cycle.
const I_LIMIT: f32 = 10.;

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

/// A PID controller.
///
/// You should call `next_control_output` at a regular interval.
#[derive(Debug)]
pub struct Pid {
    /// The accumulated I output.
    ///
    /// Units: duty cycle.
    i_sum: f32,
    /// The previous negative error.
    /// Used in the D output.
    ///
    /// Units: motor RPM.
    previous_negative_error: f32,
}

impl Pid {
    /// Creates a new PID controller.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            i_sum: 0.,
            previous_negative_error: 0.,
        }
    }

    /// Calculates the output.
    ///
    /// You should pass in `setpoint motor RPM - current motor RPM`.
    #[must_use]
    pub fn next_control_output(&mut self, negative_error: i16) -> f32 {
        // We can use algebraic operators since we do not require determinism.
        // https://doc.rust-lang.org/core/primitive.f32.html#algebraic-operators
        let negative_error = f32::from(negative_error);
        // P = K_P * error
        let p_term = negative_error.algebraic_mul(K_P);
        // I = total error + error * period * K_I
        // I is clamped to mitigate integral windup.
        // https://docs.wpilib.org/en/stable/docs/software/advanced-controls/introduction/common-control-issues.html#integral-term-windup
        // K_I and the period are constants so we can multiply them at compile time.
        self.i_sum = self
            .i_sum
            .algebraic_add(negative_error.algebraic_mul(const { K_I * LOOP_PERIOD_SECS }))
            .clamp(-I_LIMIT, I_LIMIT);
        // D = (error - previous error) / period * K_D
        // K_D and the period are constants so we can divide them at compile time.
        let d_term = negative_error
            .algebraic_sub(self.previous_negative_error)
            .algebraic_mul(const { K_D / LOOP_PERIOD_SECS });
        self.previous_negative_error = negative_error;

        p_term.algebraic_add(self.i_sum).algebraic_add(d_term)
    }

    /// Resets the previous error and I output.
    pub const fn reset_integral_output(&mut self) {
        self.previous_negative_error = 0.;
        self.i_sum = 0.;
    }
}

impl Default for Pid {
    fn default() -> Self {
        Self::new()
    }
}

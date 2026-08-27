//! This module contains our PID control code.

/// The inverse of the proportional gain.
///
/// The non-inverse of `K_P` is in units of duty cycle per motor RPM error.
pub const K_P_INVERSE: i16 = 8;

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

/// Returns the output of a basic P controller.
#[must_use]
pub const fn next_control_output(negative_error: i16) -> i16 {
    negative_error / K_P_INVERSE
}

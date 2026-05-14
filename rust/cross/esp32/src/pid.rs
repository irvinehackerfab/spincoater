//! This module contains our PID control code.

/// The inverse of the proportional gain.
///
/// The non-inverse of `K_P` is in units of duty cycle per motor RPM error.
pub const K_P_INVERSE: i16 = 16;

/// Calculates the difference between the setpoint and current RPM.
///
/// This function never fails. The parameters and result are all truncated to fit in an [`i16`].
#[must_use]
pub fn error(setpoint_rpm: u16, current_rpm: u16) -> i16 {
    let setpoint_rpm = i16::try_from(setpoint_rpm).unwrap_or(i16::MAX);
    let current_rpm = i16::try_from(current_rpm).unwrap_or(i16::MAX);
    setpoint_rpm.saturating_sub(current_rpm)
}

/// Returns the output of a basic P controller.
#[must_use]
pub fn next_control_output(error: i16) -> i16 {
    error / K_P_INVERSE
}

//! This module contains additional functionality for tracking the MCU's state.

use sc_messages::motion_profile;
use sc_messages::pwm::DutyCycle;
use serde::{Deserialize, Serialize};

/// The conversion factor from motor revolutions to plate revolutions.
const MOTOR_TO_PLATE_CONVERSION: f64 = 20.0 / 70.0;

/// A wrapper around [`motion_profile::State`] with the plate RPM added.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct State {
    /// The setpoint motor RPM.
    pub setpoint_rpm: u16,
    /// The setpoint plate RPM.
    pub setpoint_plate_rpm: f64,
    /// The measured motor RPM.
    pub current_rpm: u16,
    /// The measured plate RPM.
    pub current_plate_rpm: f64,
    /// The current duty cycle being set to try and reach the setpoint.
    pub duty_cycle: DutyCycle,
    /// The time (in micros) since the motion profile started.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    #[serde(rename = "time (micros)")]
    pub time: u64,
}

impl From<motion_profile::State> for State {
    fn from(value: motion_profile::State) -> Self {
        Self {
            setpoint_rpm: value.setpoint_rpm,
            setpoint_plate_rpm: f64::from(value.setpoint_rpm) * MOTOR_TO_PLATE_CONVERSION,
            current_rpm: value.current_rpm,
            current_plate_rpm: f64::from(value.current_rpm) * MOTOR_TO_PLATE_CONVERSION,
            duty_cycle: value.duty_cycle,
            time: value.time,
        }
    }
}

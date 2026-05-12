//! This module contains additional functionality for tracking the MCU's state.

use std::time::Duration;

use sc_messages::pwm::{DutyCycle, PERIOD};
use sc_messages::{MOTOR_REVOLUTIONS, PLATE_REVOLUTIONS};
use sc_messages::{motion_profile, touchscreen::TouchPoint};
use serde::{Deserialize, Serialize};

/// The conversion factor from motor revolutions to plate revolutions.
const MOTOR_TO_PLATE_CONVERSION: f64 = PLATE_REVOLUTIONS as f64 / MOTOR_REVOLUTIONS as f64;

/// A wrapper around both [`MotionProfileState`] and [`TouchPoint`].
#[derive(Debug, Clone, Default)]
pub struct McuState {
    pub motion_profile_state: Option<MotionProfileState>,
    pub touch_state: Option<TouchPoint>,
}

impl McuState {
    #[must_use]
    pub fn setpoint_rpm(&self) -> Option<u16> {
        self.motion_profile_state
            .as_ref()
            .map(|state| state.setpoint_rpm)
    }

    #[must_use]
    pub fn setpoint_plate_rpm(&self) -> Option<f64> {
        self.motion_profile_state
            .as_ref()
            .map(|state| state.setpoint_plate_rpm)
    }

    #[must_use]
    pub fn current_rpm(&self) -> Option<u16> {
        self.motion_profile_state
            .as_ref()
            .map(|state| state.current_rpm)
    }

    #[must_use]
    pub fn current_plate_rpm(&self) -> Option<f64> {
        self.motion_profile_state
            .as_ref()
            .map(|state| state.current_plate_rpm)
    }

    #[must_use]
    pub fn rpm_error(&self) -> Option<i16> {
        self.motion_profile_state
            .as_ref()
            .map(|state| state.rpm_error)
    }

    #[must_use]
    pub fn plate_rpm_error(&self) -> Option<f64> {
        self.motion_profile_state
            .as_ref()
            .map(|state| state.plate_rpm_error)
    }

    #[must_use]
    pub fn duty_cycle(&self) -> Option<DutyCycle> {
        self.motion_profile_state
            .as_ref()
            .map(|state| state.duty_cycle)
    }

    #[must_use]
    pub fn duty_cycle_f32(&self) -> Option<f32> {
        self.motion_profile_state
            .as_ref()
            .map(|state| f32::from(*state.duty_cycle) / f32::from(PERIOD))
    }

    #[must_use]
    pub fn time(&self) -> Option<f64> {
        self.motion_profile_state
            .as_ref()
            .map(|state| Duration::from_micros(state.time).as_secs_f64())
    }

    #[must_use]
    pub fn touch_state(&self) -> &Option<TouchPoint> {
        &self.touch_state
    }
}

/// A wrapper around [`motion_profile::State`] with the plate RPM added.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionProfileState {
    /// The setpoint motor RPM.
    pub setpoint_rpm: u16,
    /// The setpoint plate RPM.
    pub setpoint_plate_rpm: f64,
    /// The measured motor RPM.
    pub current_rpm: u16,
    /// The measured plate RPM.
    pub current_plate_rpm: f64,
    /// Setpoint motor RPM - current motor RPM.
    pub rpm_error: i16,
    /// Setpoint plate RPM - current plate RPM.
    pub plate_rpm_error: f64,
    /// The current duty cycle being set to try and reach the setpoint.
    pub duty_cycle: DutyCycle,
    /// The time (in micros) since the motion profile started.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    #[serde(rename = "time (micros)")]
    pub time: u64,
}

impl From<motion_profile::State> for MotionProfileState {
    fn from(state: motion_profile::State) -> Self {
        Self {
            setpoint_rpm: state.setpoint_rpm,
            setpoint_plate_rpm: f64::from(state.setpoint_rpm) * MOTOR_TO_PLATE_CONVERSION,
            current_rpm: state.current_rpm,
            current_plate_rpm: f64::from(state.current_rpm) * MOTOR_TO_PLATE_CONVERSION,
            rpm_error: state.rpm_error,
            plate_rpm_error: f64::from(state.rpm_error) * MOTOR_TO_PLATE_CONVERSION,
            duty_cycle: state.duty_cycle,
            time: state.time,
        }
    }
}

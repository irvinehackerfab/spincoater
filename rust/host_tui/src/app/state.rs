//! This module contains additional functionality for tracking the MCU's state.

use std::time::Duration;

use ratatui::prelude::{Frame, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Paragraph};
use sc_messages::motion_profile;
use sc_messages::pwm::{DutyCycle, PERIOD};
use sc_messages::{MOTOR_REVOLUTIONS, PLATE_REVOLUTIONS};
use serde::{Deserialize, Serialize};

/// The conversion factor from motor revolutions to plate revolutions.
const MOTOR_TO_PLATE_CONVERSION: f64 = PLATE_REVOLUTIONS as f64 / MOTOR_REVOLUTIONS as f64;

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
    /// The duty cycle from range 0.0..1.0.
    pub duty_cycle_f32: f32,
    /// The time (in micros) since the motion profile started.
    // I would like to use `embassy_time::duration::Duration`,
    // but it doesn't impl Serialize.
    #[serde(rename = "time (micros)")]
    pub time: u64,
}

impl MotionProfileState {
    /// Renders the motion profile state.
    pub fn render(&self, block: Block<'_>, area: Rect, frame: &mut Frame) {
        let paragraph = Paragraph::new(Text::from_iter([
            Line::raw(format!(
                "Time (s): {}",
                Duration::from_micros(self.time).as_secs_f64()
            )),
            Line::raw(format!("Setpoint RPM: {}", self.setpoint_rpm)),
            Line::raw(format!("Setpoint plate RPM: {}", self.setpoint_plate_rpm)),
            Line::raw(format!("Current RPM: {}", self.current_rpm)),
            Line::raw(format!("Current plate RPM: {}", self.current_plate_rpm)),
            Line::raw(format!("RPM error: {}", self.rpm_error)),
            Line::raw(format!("Plate RPM error: {}", self.plate_rpm_error)),
            Line::raw(format!("Duty Cycle (0..{PERIOD}): {}", self.duty_cycle)),
            Line::raw(format!("Duty Cycle (0.0..1.0): {}", self.duty_cycle_f32)),
        ]))
        .block(block);
        frame.render_widget(paragraph, area);
    }
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
            duty_cycle_f32: f32::from(*state.duty_cycle) / f32::from(PERIOD),
            time: state.time,
        }
    }
}

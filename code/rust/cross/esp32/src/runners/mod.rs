//! This module contains the runners we use to control the spincoater's motor.

pub mod pc;
pub mod touchscreen;

use crate::gpio::pwm::FREQUENCY;
use embassy_time::Duration;

/// The period as a u64.
pub const LOOP_PERIOD_MILLIS_U64: u64 = 20;

/// The period that the main control loop runs at.
pub const LOOP_PERIOD: Duration = Duration::from_millis(LOOP_PERIOD_MILLIS_U64);

/// There is no point in making the runner update at a greater frequency than the motor controller.
const _: () = assert!(LOOP_PERIOD.as_micros() >= FREQUENCY.as_duration().as_micros());

//! This cross-platform crate describes the message types sent between the host PC and microcontrollers.
#![no_std]

pub mod icd;
pub mod motion_profile;
pub mod pwm;
pub mod touchscreen;
pub mod vacuum_pump;

/// The number of motor revolutions per [`PLATE_REVOLUTIONS`] plate revolutions.
pub const MOTOR_REVOLUTIONS: u32 = 72;

/// The number of plate revolutions per [`MOTOR_REVOLUTIONS`] motor revolutions.
pub const PLATE_REVOLUTIONS: u32 = 30;

//! This module contains all encoder functionality.
//!
//! If you're looking for the interrupt service routine that handles hall effect sensor readings,
//! it's located in the [gpio](`crate::gpio`) module.
use core::{cell::RefCell, sync::atomic::AtomicU32};

use critical_section::Mutex;
use esp_hal::gpio::Input;

/// Provides access to the hall effect sensor.
pub static ENCODER: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

/// The counter for the motor revolutions. This counter is equal to motor revolutions * 2.
pub static MOTOR_REVOLUTIONS_DOUBLED: AtomicU32 = AtomicU32::new(0);

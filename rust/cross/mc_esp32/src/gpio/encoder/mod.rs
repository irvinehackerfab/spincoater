use core::cell::RefCell;

use critical_section::Mutex;
use esp_hal::gpio::Input;

/// Provides access to the hall effect sensor.
pub static ENCODER: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

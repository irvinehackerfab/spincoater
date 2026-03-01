//! This module contains all encoder functionality.
//!
//! If you're looking for the interrupt service routine that handles hall effect sensor readings,
//! it's located in the [gpio](`crate::gpio`) module.
use core::{
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};

use critical_section::Mutex;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use embassy_time::Timer;
use esp_hal::{gpio::Input, time::Instant};

use crate::gpio::display::terminal::channel::{
    TERMINAL_CHANNEL_SIZE, TuiEvent, send_event_or_report,
};

/// Provides access to the hall effect sensor.
pub static ENCODER: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

/// The counter for the motor revolutions. This counter is equal to motor revolutions * 2.
pub static MOTOR_REVOLUTIONS_DOUBLED: AtomicU32 = AtomicU32::new(0);

/// Every second, this task sends the number of plate revolutions per minute to the terminal.
/// It only sends the RPM if at least half a motor revolution has occurred.
///
/// # Panics
/// This task panics if more than [`u32::MAX`] milliseconds has passed after 1000 milliseconds has passed,
/// which should never happen.
///
/// It also panics if the rpm exceeds [`u32::MAX`].
#[embassy_executor::task]
pub async fn read_rpm(
    to_terminal: Sender<'static, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
) -> ! {
    let mut previous_time = Instant::now();
    loop {
        Timer::after_secs(1).await;
        // Relaxed ordering because the order of instructions does not matter for the swap.
        let motor_revolutions_doubled = MOTOR_REVOLUTIONS_DOUBLED.swap(0, Ordering::Relaxed);
        let time = previous_time.elapsed();
        if motor_revolutions_doubled != 0 {
            let time_ms =
                u32::try_from(time.as_millis()).expect("1000 milliseconds should fit in a u32.");
            // (2*motor revolutions) * 1/2 * (20 plate revolutions / 74 motor revolutions) * 1/(`time` ms) * (6000 ms / 1 min)
            // = (2*motor revolutions) * 30,000 / (37 * `time`)
            // Final units: plate revolutions per minute
            let rpm = motor_revolutions_doubled * 30_000 / (37 * (time_ms));
            let rpm = u16::try_from(rpm).expect("The rpm should never exceed 65535.");
            send_event_or_report(&to_terminal, TuiEvent::RpmValue(rpm)).await;
        }
        // Increment the time passed regardless of the revolutions counted.
        previous_time += time;
    }
}

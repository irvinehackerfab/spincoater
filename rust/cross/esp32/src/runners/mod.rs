//! This module contains the runners we use to control the spincoater's motor.

pub mod motion_profile;
pub mod rpm;

use crate::LOOP_PERIOD;
use embassy_time::{Instant, Timer};

/// Sleeps if less than [`LOOP_PERIOD`] time has passed since the last end of this function.
///
/// Returns the instant at the end of this function call.
/// This value should be passed to the next call to this method.
async fn sleep(previous_sleep_end: Instant) -> Instant {
    let elapsed_since_previous_sleep_end = previous_sleep_end.elapsed();
    // Only sleep if less than LOOP_PERIOD time has passed since the previous loop start.
    match LOOP_PERIOD.checked_sub(elapsed_since_previous_sleep_end) {
        Some(time_to_sleep) => {
            let before_sleep = Instant::now();
            Timer::after(time_to_sleep).await;
            // Manually calculating the end of the function makes this function immune to oversleep from the timer.
            before_sleep
                .checked_add(time_to_sleep)
                .expect("This program will not run for 584558 years.")
        }
        None => Instant::now(),
    }
}

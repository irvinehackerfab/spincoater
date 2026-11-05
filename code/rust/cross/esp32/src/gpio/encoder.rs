//! This module contains all encoder functionality.

use core::ops::Div;
use embassy_executor::task;
use esp_hal::{gpio::Input, time::Instant};
use esp_sync::NonReentrantMutex;
use muldiv::MulDiv;
use sc_messages::{MOTOR_REVOLUTIONS, PLATE_REVOLUTIONS};

/// Provides global access to the encoder state.
pub static ENCODER_STATE: NonReentrantMutex<EncoderState> =
    NonReentrantMutex::new(EncoderState::new(Instant::EPOCH, 0));

/// Data that is used by the encoder interrupt.
#[derive(Debug)]
pub struct EncoderState {
    /// The previous execution of the encoder interrupt.
    pub previous_time: Instant,
    /// The last recorded plate RPM.
    pub rpm: u16,
}

impl EncoderState {
    /// Creates a new encoder state.
    #[must_use]
    pub const fn new(previous_time: Instant, rpm: u16) -> Self {
        Self { previous_time, rpm }
    }

    /// Calculates the rpm between the last interrupt and now.
    ///
    /// Stores the result in the ring buffer.
    ///
    /// # Panics
    /// Panics if this method is called two times in one microsecond (which will never happen)
    pub fn calculate_rpm(&mut self, now: Instant) {
        #[allow(
            clippy::arithmetic_side_effects,
            reason = "Timer is monotonic and this program will not run for 36,558 years so the time will not overflow."
        )]
        let time_since_last_interrupt = now - self.previous_time;
        // 1 interrupt * (1 motor revolution / 2 interrupts) * 1/(`time_since_last_interrupt` μs) * (10^6 μs / 1 s) * (60 s / 1 min)
        // = 30,000,000 / `time_since_last_interrupt`
        // Final units: motor revolutions per minute
        let rpm = 30_000_000.div(
            // The time will never be 0 because the CPU cannot calculate the RPM and re-run the method in the same microsecond.
            time_since_last_interrupt
                .as_micros()
                .try_into()
                .unwrap_or(usize::MAX),
        );
        // We filter out RPMs greater than u16::MAX because it only exceeds u16::MAX when an encoder glitch occurs.
        if let Ok(rpm) = u16::try_from(rpm) {
            self.rpm = rpm;
        }
        self.previous_time = now;
    }

    /// Resets the encoder state.
    pub fn reset(&mut self) {
        self.previous_time = Instant::now();
        self.rpm = 0;
    }
}

/// Converts from plate revolutions to motor revolutions.
///
/// The return value is capped at [`u16::MAX`].
#[must_use]
pub fn plate_to_motor_revolutions(rpm: u16) -> u16 {
    rpm.mul_div_round(MOTOR_REVOLUTIONS, PLATE_REVOLUTIONS)
        .unwrap_or(u16::MAX)
}

/// Converts from motor revolutions to plate revolutions.
///
/// The return value is capped at [`u16::MAX`].
#[must_use]
pub fn motor_to_plate_revolutions(rpm: u16) -> u16 {
    rpm.mul_div_round(PLATE_REVOLUTIONS, MOTOR_REVOLUTIONS)
        .unwrap_or(u16::MAX)
}

/// The task for detecting motor revolutions.
#[task]
pub async fn detect_motor_revolutions(mut encoder: Input<'static>) -> ! {
    loop {
        encoder.wait_for_rising_edge().await;
        // Get the time immediately, before the mutex busy-waits.
        // This prevents the runner from messing with RPM calculation by accessing the mutex.
        let now = Instant::now();
        // An interrupt occurred, so it's time to calculate the rpm.
        ENCODER_STATE.with(|state| state.calculate_rpm(now));
    }
}

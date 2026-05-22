//! This module contains all encoder functionality.
//!
//! If you're looking for the interrupt service routine that handles hall effect sensor readings,
//! it's located in the [gpio](`crate::gpio`) module.
use core::sync::atomic::AtomicU32;
use esp_hal::{gpio::Input, time::Instant};
use esp_sync::NonReentrantMutex;
use heapless::HistoryBuf;
use sc_messages::{MOTOR_REVOLUTIONS, PLATE_REVOLUTIONS};

/// Provides the interrupt handler access to the encoder.
pub static ENCODER: NonReentrantMutex<Option<Input>> = NonReentrantMutex::new(None);

/// Provides the interrupt handler access to its state.
pub static ENCODER_STATE: NonReentrantMutex<EncoderState> =
    NonReentrantMutex::new(EncoderState::new(Instant::EPOCH, HistoryBuf::new()));

/// The length of the RPM ring buffer for the interrupt handler.
///
/// This is currently set to about the size that is necessary to store every RPM data point in
/// 20 milliseconds.
pub const RING_BUFFER_LENGTH: usize = 16;

/// A counter for the motor revolutions that increments by one every encoder interrupt. This counter is equal to motor revolutions * 2.
pub static MOTOR_REVOLUTIONS_DOUBLED: AtomicU32 = AtomicU32::new(0);

/// Data that is used by the encoder interrupt.
#[derive(Debug)]
pub struct EncoderState {
    /// The previous execution of the encoder interrupt.
    pub previous_time: Instant,
    /// The last [`RING_BUFFER_LENGTH`] RPM values for calculating the moving average.
    pub rpm_ring_buffer: HistoryBuf<usize, RING_BUFFER_LENGTH>,
}

impl EncoderState {
    /// Creates a new encoder state.
    #[must_use]
    pub const fn new(
        previous_time: Instant,
        rpm_ring_buffer: HistoryBuf<usize, RING_BUFFER_LENGTH>,
    ) -> Self {
        Self {
            previous_time,
            rpm_ring_buffer,
        }
    }

    /// Calculates the rpm between the last interrupt and now.
    ///
    /// Stores the result in the ring buffer.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "It's impossible for the motor RPM to exceed u16::MAX."
    )]
    pub fn calculate_rpm(&mut self) {
        let time_since_last_interrupt = self.previous_time.elapsed().as_micros();
        // 1 interrupt * (1 motor revolution / 2 interrupts) * 1/(`time_since_last_interrupt` μs) * (10^6 μs / 1 s) * (60 s / 1 min)
        // = 30,000,000 / `time_since_last_interrupt`
        // Final units: motor revolutions per minute
        // The motor RPM will never actually reach 30,000,000, so if two interrupts somehow occur at the same microsecond,
        // we just consider that to be the highest possible value.
        // We truncate here because the motor RPM will never exceed u32::MAX.
        let rpm = 30_000_000u64
            .checked_div(time_since_last_interrupt)
            .unwrap_or(u64::MAX) as usize;
        // Simple filter to remove outliers
        match self.rpm_ring_buffer.recent() {
            Some(previous_rpm) => {
                /// If you lower this, you risk being too strict and breaking the rpm calculation.
                /// If too much time passes between non-spurious interrupts,
                /// the rpm calculator will stop accepting valid rpms.
                const MAXIMUM_ALLOWED_RPM_DIFFERENCE: usize = 10_000;
                if rpm.abs_diff(*previous_rpm) < MAXIMUM_ALLOWED_RPM_DIFFERENCE {
                    self.rpm_ring_buffer.write(rpm);
                }
            }
            None => self.rpm_ring_buffer.write(rpm),
        }
        self.previous_time = Instant::now();
    }

    /// Resets the encoder state.
    pub fn reset(&mut self) {
        self.previous_time = Instant::now();
        self.rpm_ring_buffer.clear();
    }
}

/// Calculates the current rpm as a rolling average.
///
/// This function never fails. If the RPM is greater than [`u16::MAX`], [`u16::MAX`] is returned.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn calculate_rpm<const N: usize>(rpm_ring_buffer: &HistoryBuf<usize, N>) -> u16 {
    rpm_ring_buffer
        .as_slice()
        .iter()
        .sum::<usize>()
        .checked_div(rpm_ring_buffer.len())
        .unwrap_or(0) as u16
}

/// Converts from plate revolutions to motor revolutions.
///
/// The return value is truncated to fit in a [`u16`].
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn plate_to_motor_revolutions(rpm: u16) -> u16 {
    // Operate in u32 to prevent overflow
    let rpm = u32::from(rpm);
    rpm.saturating_mul(MOTOR_REVOLUTIONS)
        .strict_div(PLATE_REVOLUTIONS) as u16
}

/// Converts from motor revolutions to plate revolutions.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn motor_to_plate_revolutions(rpm: u16) -> u16 {
    // Operate in u32 to prevent overflow
    let rpm = u32::from(rpm);
    rpm.saturating_mul(PLATE_REVOLUTIONS)
        .strict_div(MOTOR_REVOLUTIONS) as u16
}

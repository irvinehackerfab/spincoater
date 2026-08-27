//! This module contains all encoder functionality.
//!
//! If you're looking for the interrupt service routine that handles hall effect sensor readings,
//! it's located in the [gpio](`crate::gpio`) module.

use core::{ops::Div, sync::atomic::AtomicU32};
use embassy_executor::task;
use embassy_time::Instant;
use esp_hal::gpio::Input;
use esp_sync::NonReentrantMutex;
use heapless::HistoryBuf;
use muldiv::MulDiv;
use sc_messages::{MOTOR_REVOLUTIONS, PLATE_REVOLUTIONS};

/// Provides global access to the encoder.
pub static ENCODER: NonReentrantMutex<Option<Input>> = NonReentrantMutex::new(None);

/// Provides global access to the encoder state.
pub static ENCODER_STATE: NonReentrantMutex<EncoderState> =
    NonReentrantMutex::new(EncoderState::new(Instant::MIN, HistoryBuf::new()));

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
    pub fn calculate_rpm(&mut self) {
        let now = Instant::now();
        // 1 interrupt * (1 motor revolution / 2 interrupts) * 1/(`time_since_last_interrupt` μs) * (10^6 μs / 1 s) * (60 s / 1 min)
        // = 30,000,000 / `time_since_last_interrupt`
        // Final units: motor revolutions per minute
        // The motor RPM will never actually reach 30,000,000, so if two interrupts somehow occur at the same microsecond,
        // we just consider the rpm to be extremely high.
        // We cap the rpm to usize::MAX here because the motor RPM will never exceed usize::MAX.
        let rpm = now.checked_duration_since(self.previous_time).map_or(
            usize::MAX,
            |time_since_last_interrupt| {
                30_000_000u64
                    .div(time_since_last_interrupt.as_micros())
                    .try_into()
                    .unwrap_or(usize::MAX)
            },
        );
        self.rpm_ring_buffer.write(rpm);
        self.previous_time = now;
    }

    /// Resets the encoder state.
    pub fn reset(&mut self) {
        self.previous_time = Instant::now();
        self.rpm_ring_buffer.clear();
    }
}

/// Calculates the current rpm as a rolling average.
///
/// The RPM is capped at [`u16::MAX`].
#[must_use]
pub fn calculate_average_rpm<const N: usize>(rpm_ring_buffer: &HistoryBuf<usize, N>) -> u16 {
    rpm_ring_buffer
        .as_slice()
        .iter()
        .fold(0, |a, b| b.saturating_add(a))
        .checked_div(rpm_ring_buffer.len())
        .unwrap_or(0)
        .try_into()
        .unwrap_or(u16::MAX)
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
///
/// Todo: Replace with [`interrupt_handler`](crate::gpio::interrupt_handler)
/// when the [Io driver bug](https://github.com/esp-rs/esp-hal/issues/5881) is fixed.
#[task]
pub async fn detect_motor_revolutions(mut encoder: Input<'static>) -> ! {
    loop {
        encoder.wait_for_rising_edge().await;
        // An interrupt occurred, so it's time to calculate the rpm.
        ENCODER_STATE.with(EncoderState::calculate_rpm);
    }
}

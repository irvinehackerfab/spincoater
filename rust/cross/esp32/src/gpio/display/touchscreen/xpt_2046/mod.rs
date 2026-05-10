//! This module contains the SPI driver for the XPT2046 touchscreen.

use core::convert::From;

use embedded_graphics::prelude::Point;
use embedded_hal::spi::{ErrorType, Operation, SpiDevice};

/// The time (in nanoseconds) we must wait before the first rising edge of the clock.
const T_CSS: u32 = 100;

/// The length of the word buffer.
const BUFFER_LENGTH: usize = 5;

/// The control byte we send when we want the X position.
///
/// Reasoning:
/// - S bit: Always high.
/// - A2-A0: 101 (Table 5)
/// - Mode: Low because we want 12 bit resolution instead of 8 bit. (Page 21)
/// - SER/DFR: Low because we want differential mode, which is "preferred" for X and Y measurements. (Page 22)
/// - PD1-PD0: 00. This is necessary if we want both the ADC on for taking measurements, and `PEN_IRQ` interrupts enabled. (Table 8)
const GET_X_POSITION: u8 = 0b1101_0000;

/// The control byte we send when we want the Y position.
///
/// Reasoning:
/// - S bit: Always high.
/// - A2-A0: 001 (Table 5)
/// - Mode: Low because we want 12 bit resolution instead of 8 bit. (Page 21)
/// - SER/DFR: Low because we want differential mode, which is "preferred" for X and Y measurements. (Page 22)
/// - PD1-PD0: 00. This is necessary if we want both the ADC on for taking measurements, and `PEN_IRQ` interrupts enabled. (Table 8)
const GET_Y_POSITION: u8 = 0b1001_0000;

/// Since we can't easily split reads into multiple commands, we need to perform simultaneous reads and writes.
/// If we shift first command right by 3 bits and read the 5th bit of the X position at the same time as we propagate the second S,
/// the X and Y position will be aligned to the 3rd and 5th bytes respectively.
// (If you're in the esp32 repo, I drew a timing diagram for this transaction in the images folder.)
const FULL_COMMAND: [u8; BUFFER_LENGTH] = [
    GET_X_POSITION >> 3,
    GET_X_POSITION << 5,
    GET_Y_POSITION >> 3,
    GET_Y_POSITION << 5,
    0,
];

/// The SPI driver for the XPT2046 touchscreen.
pub struct Xpt2046<D> {
    /// The SPI device.
    spi: D,
    /// The buffer for receiving words.
    buffer: [u8; BUFFER_LENGTH],
}

impl<D> Xpt2046<D> {
    /// Creates a new touchscreen device.
    ///
    /// The SPI clock frequency should be <= 5 MHz.
    ///
    /// CPOL and CPHA must be 0.
    #[must_use]
    pub fn new(spi: D) -> Self {
        Self {
            spi,
            buffer: [0; BUFFER_LENGTH],
        }
    }

    /// Returns the point where the screen was touched.
    ///
    /// The values are in the range 0..4095.
    ///
    /// # Errors
    /// Returns an error if the SPI transaction fails.
    pub fn point(&mut self) -> Result<Point, <D as ErrorType>::Error>
    where
        D: SpiDevice,
    {
        // I'm ignoring the propagation delay "tDO". Hopefully that's ok.
        self.spi.transaction(&mut [
            Operation::DelayNs(T_CSS),
            Operation::Transfer(&mut self.buffer, &FULL_COMMAND),
        ])?;

        let x = i32::from(self.buffer[1]) << 8 | i32::from(self.buffer[2]);
        let y = i32::from(self.buffer[3]) << 8 | i32::from(self.buffer[4]);

        Ok(Point::new(x, y))
    }
}

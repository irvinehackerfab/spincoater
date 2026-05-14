//! This module contains the SPI driver for the XPT2046 touchscreen.

use core::convert::From;

use embedded_graphics_core::geometry::Point;
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
/// - SER/DFR: Low because we want differential mode, which is "preferred" for X/Y/Z measurements. (Page 22)
/// - PD1-PD0: 00. This is necessary if we want both the ADC on for taking measurements, and `PEN_IRQ` interrupts enabled. (Table 8)
const GET_X_POSITION: u8 = 0b1101_0000;

/// The control byte we send when we want the Y position.
///
/// Reasoning:
/// - S bit: Always high.
/// - A2-A0: 001 (Table 5)
/// - Mode: Low because we want 12 bit resolution instead of 8 bit. (Page 21)
/// - SER/DFR: Low because we want differential mode, which is "preferred" for X/Y/Z measurements. (Page 22)
/// - PD1-PD0: 00. This is necessary if we want both the ADC on for taking measurements, and `PEN_IRQ` interrupts enabled. (Table 8)
const GET_Y_POSITION: u8 = 0b1001_0000;

// /// The control byte we send when we want Z1.
// ///
// /// Reasoning:
// /// - S bit: Always high.
// /// - A2-A0: 011 (Table 5)
// /// - Mode: Low because we want 12 bit resolution instead of 8 bit. (Page 21)
// /// - SER/DFR: Low because we want differential mode, which is "preferred" for X/Y/Z measurements. (Page 22)
// /// - PD1-PD0: 00. This is necessary if we want both the ADC on for taking measurements, and `PEN_IRQ` interrupts enabled. (Table 8)
// const GET_Z1_POSITION: u8 = 0b1011_0000;

// /// The control byte we send when we want Z2.
// ///
// /// Reasoning:
// /// - S bit: Always high.
// /// - A2-A0: 100 (Table 5)
// /// - Mode: Low because we want 12 bit resolution instead of 8 bit. (Page 21)
// /// - SER/DFR: Low because we want differential mode, which is "preferred" for X/Y/Z measurements. (Page 22)
// /// - PD1-PD0: 00. This is necessary if we want both the ADC on for taking measurements, and `PEN_IRQ` interrupts enabled. (Table 8)
// const GET_Z2_POSITION: u8 = 0b1100_0000;

/// Since we can't easily split reads into multiple commands, we need to perform simultaneous reads and writes.
/// If we shift first command right by 3 bits and read the 5th bit of the X position at the same time as we propagate the second S,
/// the data will be aligned to the 3rd and 5th bytes respectively.
// (If you're in the esp32 repo, I drew a timing diagram for this transaction in the images folder.)
const FULL_COMMAND: [u8; BUFFER_LENGTH] = [
    GET_X_POSITION >> 3,
    GET_X_POSITION << 5,
    GET_Y_POSITION >> 3,
    GET_Y_POSITION << 5,
    0,
];

/// Sending a control byte with PD0 low enables `PEN_IRQ`.
/// We need 3 bytes because `PEN_IRQ` isn't enabled until the the end of the conversion,
/// which is the falling edge after bit 1 of the data is clocked out of the XPT.
const INIT_COMMAND: [u8; 3] = [0x80, 0, 0];

/// The constant that converts values from range 330..3701 to 0..3371.
const MIN_X: i32 = 330;

/// The constant that converts values from range 364..3722 to 0..3358.
const MIN_Y: i32 = 364;

/// The numerator that converts values from range 0..3371 to 0..4095.
const LERP_NUMERATOR: i32 = 4_095;

/// The denominator that converts values from range 0..3371 to 0..4095.
const X_DENOMINATOR: i32 = 3_371;

/// The denominator that converts values from range 0..3358 to 0..4095.
const Y_DENOMINATOR: i32 = 3_358;

/// The highest possible x or y value.
pub const MAX_VALUE: u16 = 4095;

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
}

impl<D> Xpt2046<D>
where
    D: SpiDevice,
{
    /// Enables `PEN_IRQ` by sending a control byte to power down the device.
    ///
    /// # Errors
    /// Returns an error if the SPI transaction fails.
    pub fn enable_interrupt(&mut self) -> Result<(), <D as ErrorType>::Error> {
        self.spi
            .transaction(&mut [Operation::DelayNs(T_CSS), Operation::Write(&INIT_COMMAND)])
    }

    /// Returns the point where the screen was touched.
    ///
    /// The X and Y values are in the range 0..4095.
    ///
    /// # Errors
    /// Returns an error if the SPI transaction fails.
    pub fn point(&mut self) -> Result<Point, <D as ErrorType>::Error> {
        // I'm ignoring the propagation delay "tDO". Hopefully that's ok.
        self.spi.transaction(&mut [
            Operation::DelayNs(T_CSS),
            Operation::Transfer(&mut self.buffer, &FULL_COMMAND),
        ])?;

        let x = i32::from(self.buffer[1]) << 8 | i32::from(self.buffer[2]);
        let x = lerp_x(x);

        let y = i32::from(self.buffer[3]) << 8 | i32::from(self.buffer[4]);
        let y = lerp_y(y);

        Ok(Point::new(x, y))
    }
}

/// Linearly interpolates the x value from 330..3701 to 0..4095.
fn lerp_x(x: i32) -> i32 {
    x.saturating_sub(MIN_X)
        .max(0)
        .saturating_mul(LERP_NUMERATOR)
        .strict_div(X_DENOMINATOR)
        .min(4095)
}

/// Linearly interpolates the y value from 364..3722 to 0..4095.
fn lerp_y(y: i32) -> i32 {
    y.saturating_sub(MIN_Y)
        .max(0)
        .saturating_mul(LERP_NUMERATOR)
        .strict_div(Y_DENOMINATOR)
        .min(4095)
}

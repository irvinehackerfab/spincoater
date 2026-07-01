//! This module contains the functionality for the touchscreen.

use embassy_executor::task;

use embedded_hal::spi::ErrorType;
use embedded_hal_bus::spi::RefCellDevice;
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, Output},
    spi::master::SpiDmaBus,
};
use esp_println::println;
use static_cell::ConstStaticCell;
use xpt2046_rs::{
    Xpt2046,
    builder::{BUFFER_SIZE, Builder, Calibration as CalibrationMode, Interrupt, Raw},
    calibration::Coefficients,
};

use crate::gpio::display::{
    DisplayType,
    terminal::channel::{TerminalSender, TuiEvent},
};

/// The type of SPI device the touchscreen uses.
pub type Device<'a> = RefCellDevice<'a, SpiDmaBus<'a, Blocking>, Output<'a>, Delay>;

/// The buffer for the XPT.
pub static XPT_BUFFER: ConstStaticCell<[u8; BUFFER_SIZE]> = ConstStaticCell::new([0; _]);

/// The maximum Z resistance which is considered a valid press.
///
/// This is a low enough value to filter out unwanted inputs,
/// while still allowing presses from small objects like styluses.
pub const MAX_RESISTANCE: f32 = 400.;

/// The coefficients used by the [`Xpt2046`] to convert touchscreen coordinates to display coordinates.
pub const COEFFICIENTS: Coefficients = Coefficients {
    alpha_x: 0.002_085_868_3,
    beta_x: 0.090_170_346,
    delta_x: -24.611_855,
    alpha_y: 0.057_622_11,
    beta_y: -0.009_044_195,
    delta_y: 30.097_515,
};

/// A typestate of the [`Touchscreen`].
///
/// The touchscreen will only allow printing touch resistance values.
pub struct Test<'a> {
    /// The touchscreen driver.
    xpt: Xpt2046<'a, Device<'a>, Raw>,
    /// The pin for detecting touches.
    pen_irq: Input<'a>,
}

/// A typestate of the [`Touchscreen`].
///
/// The touchscreen will only allow calibration.
pub struct Calibration<'a> {
    xpt: Xpt2046<'a, Device<'a>, CalibrationMode<'a, DisplayType, Input<'a>>>,
}

/// A typestate of the [`Touchscreen`].
///
/// The touchscreen will allow sending touches to the terminal.
pub struct Normal<'a> {
    xpt: Xpt2046<'a, Device<'a>, Interrupt<Input<'a>>>,
    to_terminal: TerminalSender,
}

/// The touchscreen.
///
/// Its functionality depends on the mode you choose to initialize it in.
pub struct Touchscreen<M> {
    mode: M,
}

impl<'a> Touchscreen<Test<'a>> {
    /// Creates a touchscreen for testing only.
    ///
    /// # Errors
    /// Returns an error if the SPI transaction fails.
    pub fn new_test(
        spi: Device<'a>,
        buffer: &'a mut [u8; BUFFER_SIZE],
        pen_irq: Input<'a>,
    ) -> Result<Self, <Device<'a> as ErrorType>::Error> {
        let xpt = Builder::new().try_init(spi, buffer)?;
        Ok(Self {
            mode: Test { xpt, pen_irq },
        })
    }

    /// Prints the touch resistance whenever there is contact with the touchscreen.
    pub async fn report_resistance(&mut self) -> ! {
        loop {
            // Wait for any touch.
            self.mode.pen_irq.wait_for_low().await;
            // Get the touch resistance
            let result = self.mode.xpt.resistance();
            println!("Touch: {:?}", result);
        }
    }
}

impl<'a> Touchscreen<Calibration<'a>> {
    /// Creates a touchscreen for calibration only.
    ///
    /// # Errors
    /// Returns an error if the SPI transaction fails.
    pub fn new_calibration(
        spi: Device<'a>,
        buffer: &'a mut [u8; BUFFER_SIZE],
        display: &'a mut DisplayType,
        pen_irq: Input<'a>,
    ) -> Result<Self, <Device<'a> as ErrorType>::Error> {
        let xpt = Builder::new()
            .into_calibration(display, pen_irq)
            .try_init(spi, buffer)?;
        Ok(Self {
            mode: Calibration { xpt },
        })
    }

    /// Gets the coefficients of the touchscreen and prints them.
    ///
    /// This method requires real-world interaction with the touchscreen.
    /// You will have to press each of the 3 targets that show up, ideally with a stylus.
    pub async fn send_coefficients(&mut self) {
        // Get coefficients
        let result = self.mode.xpt.three_point_calibration(MAX_RESISTANCE).await;
        println!("Coefficients: {:#?}", result);
    }
}

impl<'a> Touchscreen<Normal<'a>> {
    /// Creates a new touchscreen for normal use.
    ///
    /// # Errors
    /// Returns an error if the SPI transaction fails.
    pub fn new(
        spi: Device<'a>,
        buffer: &'a mut [u8; BUFFER_SIZE],
        pen_irq: Input<'a>,
        to_terminal: TerminalSender,
    ) -> Result<Self, <Device<'a> as ErrorType>::Error> {
        let xpt = Builder::new()
            .into_interrupt(pen_irq)
            .with_coefficients(&COEFFICIENTS)
            .try_init(spi, buffer)?;
        Ok(Self {
            mode: Normal { xpt, to_terminal },
        })
    }

    /// Runs the touchscreen loop, getting points and sending them to the terminal.
    async fn handle_presses(&mut self) {
        loop {
            // Wait for hard press
            let point = match self
                .mode
                .xpt
                .wait_for_hard_press(true, MAX_RESISTANCE)
                .await
            {
                Ok(point) => point,
                Err(err) => {
                    println!("Failed to get press: {err:?}.");
                    continue;
                }
            };
            // Simple filter to remove spurious interrupts
            if point.y < 10 {
                println!("Invalid touch: {point:?}");
            } else {
                println!("Touch: {point:?}");
                // Send point
                self.mode.to_terminal.send(TuiEvent::Point(point)).await;
            }
        }
    }
}

/// Runs the touchscreen loop.
#[task]
pub async fn run_touchscreen(mut touchscreen: Touchscreen<Normal<'static>>) {
    touchscreen.handle_presses().await;
}

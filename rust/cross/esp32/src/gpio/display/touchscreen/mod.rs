//! This module contains the functionality for the touchscreen.

use embassy_executor::task;

use embassy_time::{Duration, Timer};
use embedded_hal::spi::ErrorType;
use embedded_hal_bus::spi::RefCellDevice;
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, Output},
    spi::master::SpiDmaBus,
};
use sc_messages::touchscreen::TouchPoint;

use crate::gpio::display::{
    terminal::channel::{TerminalSender, TuiEvent},
    touchscreen::xpt_2046::Xpt2046,
};
use esp_println::println;

pub mod xpt_2046;

/// The type of SPI device the touchscreen uses.
pub type Device<'a> = RefCellDevice<'a, SpiDmaBus<'a, Blocking>, Output<'a>, Delay>;

/// The debounce time for the touchscreen in milliseconds.
const DEBOUNCE: Duration = Duration::from_millis(250);

/// The touchscreen.
pub struct Touchscreen<'a> {
    xpt_2046: Xpt2046<Device<'a>>,
    pen_irq: Input<'a>,
    to_terminal: TerminalSender,
}

impl<'a> Touchscreen<'a> {
    /// Creates the touchscreen and enables the pen interrupt on the XPT.
    ///
    /// # Errors
    /// Returns an error if the pen interrupt could not be enabled.
    pub fn new(
        mut xpt_2046: Xpt2046<Device<'a>>,
        pen_irq: Input<'a>,
        to_terminal: TerminalSender,
    ) -> Result<Self, <Device<'a> as ErrorType>::Error> {
        xpt_2046.enable_interrupt()?;
        Ok(Self {
            xpt_2046,
            pen_irq,
            to_terminal,
        })
    }

    /// Runs the touchscreen loop, getting the touch point every time the touchscreen is pressed.
    async fn handle_presses(&mut self) {
        loop {
            // Wait for a new touch.
            // It is recommended that the processor mask the interrupt PENIRQ is associated with whenever the processor sends
            // a control byte to the XPT2046. This prevents false triggering of interrupts when the PENIRQ output is disabled in
            // the cases discussed in page 25 of https://www.buydisplay.com/download/ic/XPT2046.pdf.
            // That is why we are free to use this method, which stops listening after the falling edge.
            self.pen_irq.wait_for_falling_edge().await;
            let point = match self.xpt_2046.point() {
                Ok(point) => TouchPoint::from(point),
                Err(err) => {
                    println!("Error getting touch point: {err:?}");
                    continue;
                }
            };
            // Filter out screen releases by detecting x = 0
            if point.x != 0 {
                self.to_terminal
                    .send(TuiEvent::Touch(point.transpose()))
                    .await;
                // Attempt to filter out spurious interrupts
                Timer::after(DEBOUNCE).await;
            }
        }
    }

    // /// Runs the touchscreen loop, getting touch points while the touchscreen is pressed.
    // async fn handle_any_contact(&mut self) {
    //     loop {
    //         // Wait for any touch.
    //         // It is recommended that the processor mask the interrupt PENIRQ is associated with whenever the processor sends
    //         // a control byte to the XPT2046. This prevents false triggering of interrupts when the PENIRQ output is disabled in
    //         // the cases discussed in page 25 of https://www.buydisplay.com/download/ic/XPT2046.pdf.
    //         // That is why we are free to use this method, which stops listening after it sees low.
    //         self.pen_irq.wait_for_low().await;
    //         let point = match self.xpt_2046.point() {
    //             Ok(point) => TouchPoint::from(point),
    //             Err(err) => {
    //                 println!("Error getting touch point: {err:?}");
    //                 continue;
    //             }
    //         };
    //         self.to_terminal
    //             .send(TuiEvent::Touch(point.transpose()))
    //             .await;
    //     }
    // }
}

/// Runs the touchscreen loop.
#[task]
pub async fn run_touchscreen(mut touchscreen: Touchscreen<'static>) {
    touchscreen.handle_presses().await;
}

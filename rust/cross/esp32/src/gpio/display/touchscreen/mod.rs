//! This module contains the functionality for the touchscreen.

use core::convert::Infallible;

use embassy_executor::task;
use embedded_hal_bus::spi::{DeviceError, RefCellDevice};
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, Output},
    spi::{Error, master::Spi},
};
use postcard_rpc::server::Sender;
use sc_messages::{icd::TouchPointTopic, touchscreen::TouchPoint};

use crate::{
    gpio::display::touchscreen::xpt_2046::Xpt2046,
    rpc::{SEQUENCE_NUMBER, WireTx},
};

pub mod xpt_2046;

/// The type of SPI device the touchscreen uses.
pub type Device<'a> = RefCellDevice<'a, Spi<'a, Blocking>, Output<'a>, Delay>;

/// The touchscreen.
pub struct Touchscreen<'a> {
    xpt_2046: Xpt2046<Device<'a>>,
    pen_irq: Input<'a>,
    to_server: Sender<WireTx>,
}

impl<'a> Touchscreen<'a> {
    /// Creates the touchscreen.
    #[must_use]
    pub fn new(
        xpt_2046: Xpt2046<Device<'a>>,
        pen_irq: Input<'a>,
        to_server: Sender<WireTx>,
    ) -> Self {
        Self {
            xpt_2046,
            pen_irq,
            to_server,
        }
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
            let Ok(point) = self.xpt_2046.point::<DeviceError<Error, Infallible>>() else {
                let _ = self.to_server.log_str("Failed to get touch point.").await;
                continue;
            };
            let point = TouchPoint::from(point);
            let _ = self
                .to_server
                .publish::<TouchPointTopic>(SEQUENCE_NUMBER, &point)
                .await;
        }
    }

    /// Runs the touchscreen loop, getting touch points while the touchscreen is pressed.
    async fn handle_any_contact(&mut self) {
        loop {
            // Wait for any touch.
            // It is recommended that the processor mask the interrupt PENIRQ is associated with whenever the processor sends
            // a control byte to the XPT2046. This prevents false triggering of interrupts when the PENIRQ output is disabled in
            // the cases discussed in page 25 of https://www.buydisplay.com/download/ic/XPT2046.pdf.
            // That is why we are free to use this method, which stops listening after it sees low.
            self.pen_irq.wait_for_low().await;
            let Ok(point) = self.xpt_2046.point::<DeviceError<Error, Infallible>>() else {
                let _ = self.to_server.log_str("Failed to get touch point.").await;
                continue;
            };
            let point = TouchPoint::from(point);
            let _ = self
                .to_server
                .publish::<TouchPointTopic>(SEQUENCE_NUMBER, &point)
                .await;
        }
    }
}

/// Runs the touchscreen loop.
#[task]
pub async fn run_touchscreen(mut touchscreen: Touchscreen<'static>) {
    touchscreen.handle_presses().await;
}

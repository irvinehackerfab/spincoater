//! This module contains all display functionality.

pub mod terminal;
pub mod touchscreen;

use core::cell::RefCell;

use embedded_hal_bus::spi::RefCellDevice;
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::Output,
    spi::{Mode, master::SpiDma},
    time::Rate,
};
use mipidsi::{
    Display,
    interface::SpiInterface,
    models::ILI9341Rgb565,
    options::{Orientation, Rotation},
};
use static_cell::{ConstStaticCell, StaticCell};

/// The size of the buffers used for SPI.
pub const SPI_BUFFER_SIZE: usize = 32736;

/// The buffer used for display pixels.
pub static SPI_BUFFER: ConstStaticCell<[u8; SPI_BUFFER_SIZE]> = ConstStaticCell::new([0u8; _]);

/// The clock rate used for the SPI bus.
///
/// Although the ILI9341 can handle 4 MHz, the XPT2046 can only handle [2MHz](xpt2046_rs::builder::Builder::try_init).
pub const SPI_CLOCK_RATE: Rate = Rate::from_mhz(2);

/// The mode for SPI communication.
///
/// The XPT2046 requires [CPOL and CPHA](https://en.wikipedia.org/wiki/Serial_Peripheral_Interface#Clock_polarity_and_phase) to be 0.
pub const SPI_MODE: Mode = Mode::_0;

/// The entire type of the display as a type alias, so it can be reused.
pub type DisplayType = Display<
    SpiInterface<
        'static,
        RefCellDevice<'static, SpiDma<'static, Blocking>, Output<'static>, Delay>,
        Output<'static>,
    >,
    ILI9341Rgb565,
    Output<'static>,
>;

/// The static cell for the SPI bus.
pub static SPI: StaticCell<RefCell<SpiDma<'static, Blocking>>> = StaticCell::new();

/// The static cell for the display.
pub static DISPLAY: StaticCell<DisplayType> = StaticCell::new();

/// The display width of the ILI9341 in landscape mode.
pub const WIDTH: i32 = 320;

/// The display height of the ILI9341 in landscape mode.
pub const HEIGHT: i32 = 240;

/// The orientation settings for mipidsi.
pub const ORIENTATION: Orientation = Orientation::new().flip_vertical().rotate(Rotation::Deg270);

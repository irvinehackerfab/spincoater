//! This module contains all display functionality.
pub mod terminal;
pub mod touchscreen;

use core::cell::RefCell;

use embedded_hal_bus::spi::RefCellDevice;
use esp_hal::{Blocking, delay::Delay, gpio::Output, spi::master::SpiDmaBus};
use mipidsi::{
    Display,
    interface::SpiInterface,
    models::ILI9341Rgb565,
    options::{Orientation, Rotation},
};
use static_cell::{ConstStaticCell, StaticCell};

/// The size of the buffers used for SPI.
pub const SPI_BUFFER_SIZE: usize = 32768;

/// The buffer used for display pixels.
pub static SPI_BUFFER: ConstStaticCell<[u8; SPI_BUFFER_SIZE]> = ConstStaticCell::new([0u8; _]);

/// The entire type of the display as a type alias, so it can be reused.
pub type DisplayType = Display<
    SpiInterface<
        'static,
        RefCellDevice<'static, SpiDmaBus<'static, Blocking>, Output<'static>, Delay>,
        Output<'static>,
    >,
    ILI9341Rgb565,
    Output<'static>,
>;

/// The static cell for the SPI bus.
pub static SPI: StaticCell<RefCell<SpiDmaBus<'static, Blocking>>> = StaticCell::new();

/// The static cell for the display.
pub static DISPLAY: StaticCell<DisplayType> = StaticCell::new();

/// The orientation settings for mipidsi.
pub const ORIENTATION: Orientation = Orientation::new().flip_vertical().rotate(Rotation::Deg270);

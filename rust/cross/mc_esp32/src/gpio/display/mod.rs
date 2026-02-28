//! This module contains all display functionality.
pub mod terminal;

use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_hal::{Async, gpio::Output, spi::master::Spi};
use mipidsi::{Display, interface::SpiInterface, models::ILI9341Rgb565, options::Orientation};
use static_cell::{ConstStaticCell, StaticCell};

/// The buffer used for display pixels
pub static SPI_BUFFER: ConstStaticCell<[u8; 512]> = ConstStaticCell::new([0u8; 512]);

/// The entire type of the display as a type alias, so it can be reused.
pub type DisplayType = Display<
    SpiInterface<
        'static,
        ExclusiveDevice<Spi<'static, Async>, Output<'static>, NoDelay>,
        Output<'static>,
    >,
    ILI9341Rgb565,
    Output<'static>,
>;

/// The static cell for the display.
pub static DISPLAY: StaticCell<DisplayType> = StaticCell::new();

/// The orientation settings for mipidsi.
pub const ORIENTATION: Orientation = Orientation::new().flip_vertical();

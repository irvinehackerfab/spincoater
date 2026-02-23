use embedded_hal_bus::spi::{CriticalSectionDevice, NoDelay};
use esp_hal::{Async, gpio::Output, spi::master::Spi};
use mipidsi::{Display, interface::SpiInterface, models::ILI9341Rgb565};
use mousefood::{EmbeddedBackend, prelude::Rgb565};
use ratatui::Terminal;
use static_cell::ConstStaticCell;

/// The buffer used for display pixels
pub static SPI_BUFFER: ConstStaticCell<[u8; 512]> = ConstStaticCell::new([0u8; 512]);

/// This task
#[embassy_executor::task]
pub async fn update_display(
    terminal: &'static mut Terminal<
        EmbeddedBackend<
            'static,
            Display<
                SpiInterface<
                    'static,
                    CriticalSectionDevice<'static, Spi<'static, Async>, Output<'static>, NoDelay>,
                    Output<'static>,
                >,
                ILI9341Rgb565,
                Output<'static>,
            >,
            Rgb565,
        >,
    >,
) {
    todo!("terminal.draw()")
}

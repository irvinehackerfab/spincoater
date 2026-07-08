#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::cell::RefCell;

use defmt::info;
use embassy_executor::Spawner;
use embedded_hal_bus::spi::RefCellDevice;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::{DriveStrength, Input, InputConfig, Level, Output, OutputConfig, Pull},
    spi::master::{Config, Spi, SpiDmaBus},
    timer::timg::TimerGroup,
};
use esp_println as _;
use esp32::gpio::display::{
    DISPLAY, ORIENTATION, SPI, SPI_BUFFER, SPI_BUFFER_SIZE, SPI_CLOCK_RATE, SPI_MODE,
    touchscreen::{Touchscreen, XPT_BUFFER},
};
use mipidsi::{interface::SpiInterface, models::ILI9341Rgb565};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "main is the only place you should be allowed to allocate large buffers."
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO0
    // - GPIO2
    // - GPIO5
    // - GPIO12
    // - GPIO15
    // These GPIO pins are in use by some feature of the module and should not be used.
    let _ = peripherals.GPIO6;
    let _ = peripherals.GPIO7;
    let _ = peripherals.GPIO8;
    let _ = peripherals.GPIO9;
    let _ = peripherals.GPIO10;
    let _ = peripherals.GPIO11;
    let _ = peripherals.GPIO16;
    let _ = peripherals.GPIO20;

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    // ESC Workaround
    let _ = Output::new(
        peripherals.GPIO15,
        Level::High,
        OutputConfig::default().with_drive_strength(DriveStrength::_20mA),
    );

    // Initialize SPI
    let spi = SPI.init_with(|| {
        // See https://esp32.implrust.com/tft-display/circuit.html for a tutorial.
        let spi = Spi::new(
            peripherals.SPI2,
            Config::default()
                .with_frequency(SPI_CLOCK_RATE)
                .with_mode(SPI_MODE),
        )
        .expect("Frequency is within 70kHz..80MHz")
        // Master In Slave Out. SPI read line from the display to the microcontroller.
        .with_miso(peripherals.GPIO35)
        // Master Out Slave In. This is the SPI data line from the microcontroller to the display. Used to send pixel data and commands.
        .with_mosi(peripherals.GPIO33)
        // Serial Clock. SPI clock signal from the microcontroller. It synchronizes the data being sent.
        .with_sck(peripherals.GPIO32)
        .with_dma(peripherals.DMA_SPI2);
        let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(SPI_BUFFER_SIZE);
        let dma_rx_buf =
            DmaRxBuf::new(rx_descriptors, rx_buffer).expect("Failed to create DMA RX buf");
        let dma_tx_buf =
            DmaTxBuf::new(tx_descriptors, tx_buffer).expect("Failed to create DMA TX buf");
        let spi = SpiDmaBus::new(spi, dma_rx_buf, dma_tx_buf);
        RefCell::new(spi)
    });

    // Initialize display
    let display = DISPLAY.init_with(|| {
        // Chip Select. This tells the display when it should listen to SPI commands. Keep it low (active) when sending data.
        // [`RefCellDevice::new`] says to have an initial output of high.
        let cs = Output::new(peripherals.GPIO19, Level::High, OutputConfig::default());
        // Data/Command control pin. Set high to send data, low to send commands. Used to switch between writing commands and pixel data.
        let dc = Output::new(peripherals.GPIO25, Level::Low, OutputConfig::default());
        // Resets the display. Useful during startup to make sure the display starts in a known state.
        // According to [mipidsi::Builder::reset_pin], this should start high.
        // However, according to page 225 of https://www.lcdwiki.com/res/MSP2807/ILI9341%20Datasheet.pdf
        // the starting state doesn't matter.
        let reset = Output::new(peripherals.GPIO18, Level::High, OutputConfig::default());
        let spi_device = RefCellDevice::new(spi, cs, Delay::new()).expect("cs is already high");
        let interface = SpiInterface::new(spi_device, dc, SPI_BUFFER.take());
        mipidsi::Builder::new(ILI9341Rgb565, interface)
            .reset_pin(reset)
            .orientation(ORIENTATION)
            .init(&mut Delay::new())
            .expect("Failed to init display")
    });

    let t_cs = Output::new(peripherals.GPIO16, Level::High, OutputConfig::default());
    let spi = RefCellDevice::new(spi, t_cs, Delay::new()).expect("cs is already high");
    let pen_irq = Input::new(
        peripherals.GPIO34,
        // pull up because active low
        InputConfig::default().with_pull(Pull::Up),
    );
    let mut touchscreen = Touchscreen::new_calibration(spi, XPT_BUFFER.take(), display, pen_irq)
        .expect("Failed to initialize touchscreen");

    loop {
        touchscreen.send_coefficients().await;
    }
}

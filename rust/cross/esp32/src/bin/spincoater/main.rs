#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::cell::RefCell;

use embassy_executor::Spawner;
use embedded_hal_bus::spi::RefCellDevice;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Event, Input, InputConfig, Io, Level, Output, OutputConfig, Pull},
    interrupt::software::SoftwareInterruptControl,
    mcpwm::{McPwm, PeripheralClockConfig, operator::PwmPinConfig, timer::PwmWorkingMode},
    spi::master::{Config, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp32::{
    SECOND_CORE_STACK,
    gpio::{
        display::{
            DISPLAY, ORIENTATION, SPI, SPI_BUFFER,
            terminal::{TERMINAL, TerminalState, channel::TERMINAL_CHANNEL, update_terminal},
            touchscreen::{Touchscreen, run_touchscreen, xpt_2046::Xpt2046},
        },
        encoder::ENCODER,
        interrupt_handler,
        pwm::{FREQUENCY, PERIOD, PERIPHERAL_CLOCK_PRESCALER},
    },
    runners::rpm::{Runner, channel::RUNNER_CHANNEL},
};
use ibm437::IBM437_9X14_REGULAR;
use mipidsi::{interface::SpiInterface, models::ILI9341Rgb565};
use mousefood::{ColorTheme, EmbeddedBackend, EmbeddedBackendConfig};
use panic_rtt_target as _;
use ratatui::Terminal;
use sc_messages::pwm::STOP_DUTY;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "main is the only place you should be allowed to allocate large buffers."
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    rtt_init_defmt!();

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
    // let _ = peripherals.GPIO6;
    // let _ = peripherals.GPIO7;
    // let _ = peripherals.GPIO8;
    // let _ = peripherals.GPIO9;
    // let _ = peripherals.GPIO10;
    // let _ = peripherals.GPIO11;

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    // Run the encoder task/ISR on the second core so it doesn't block the program.
    let software_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        software_interrupts.software_interrupt0,
        software_interrupts.software_interrupt1,
        SECOND_CORE_STACK.take(),
        || {
            // Set the interrupt handler for GPIO.
            let mut io = Io::new(peripherals.IO_MUX);
            io.set_interrupt_handler(interrupt_handler);

            // Initialize encoder pin
            let mut encoder = Input::new(
                peripherals.GPIO27,
                InputConfig::default().with_pull(Pull::Down),
            );

            // Start listening for rising edges
            ENCODER.with(|encoder_memory_cell| {
                encoder.listen(Event::RisingEdge);
                encoder_memory_cell.replace(encoder);
            });
        },
    );

    // Initialize PWM
    let clock_cfg = PeripheralClockConfig::with_prescaler(PERIPHERAL_CLOCK_PRESCALER);
    let mut mcpwm = McPwm::new(peripherals.MCPWM0, clock_cfg);
    mcpwm.operator0.set_timer(&mcpwm.timer0);
    // connect operator0 to pin IO26:
    // https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#j3
    let mut pwm_pin = mcpwm
        .operator0
        .with_pin_a(peripherals.GPIO26, PwmPinConfig::UP_ACTIVE_HIGH);
    // start timer with timestamp values in the range that we choose.
    let timer_clock_cfg = clock_cfg
        .timer_clock_with_frequency(PERIOD, PwmWorkingMode::Increase, FREQUENCY)
        .expect("Failed to create TimerClockConfig");
    mcpwm.timer0.start(timer_clock_cfg);
    pwm_pin.set_timestamp(STOP_DUTY);

    // Initialize vacuum pump pin
    let vacuum_pump_pin = Output::new(peripherals.GPIO17, Level::Low, OutputConfig::default());

    // Initialize SPI
    let spi = SPI.init_with(|| {
        // See https://esp32.implrust.com/tft-display/circuit.html for a tutorial.
        let spi = Spi::new(
            peripherals.SPI2,
            Config::default()
                .with_frequency(Rate::from_mhz(4))
                .with_mode(esp_hal::spi::Mode::_0),
        )
        .expect("Frequency is within 70kHz..80MHz")
        // Master In Slave Out. SPI read line from the display to the microcontroller.
        .with_miso(peripherals.GPIO35)
        // Master Out Slave In. This is the SPI data line from the microcontroller to the display. Used to send pixel data and commands.
        .with_mosi(peripherals.GPIO33)
        // Serial Clock. SPI clock signal from the microcontroller. It synchronizes the data being sent.
        .with_sck(peripherals.GPIO32);
        RefCell::new(spi)
    });

    // Initialize the display
    // init_with constructs the value in-place to save stack space.
    let terminal = TERMINAL.init_with(|| {
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
        let config = EmbeddedBackendConfig {
            // The default font is too small so we use a bigger (and more optimzied) one
            font_regular: IBM437_9X14_REGULAR,
            color_theme: ColorTheme::tokyo_night(),
            ..EmbeddedBackendConfig::default()
        };
        let backend = EmbeddedBackend::new(display, config);
        Terminal::new(backend).expect("Failed to create terminal")
    });

    // Setup terminal
    let terminal_channel = TERMINAL_CHANNEL.take();

    let runner_channel = RUNNER_CHANNEL.take();

    let terminal_state = TerminalState::new(
        vacuum_pump_pin,
        terminal_channel.receiver(),
        runner_channel.sender(),
    );

    // Initialize the touchscreen
    let t_cs = Output::new(peripherals.GPIO16, Level::High, OutputConfig::default());
    let spi_device = RefCellDevice::new(spi, t_cs, Delay::new()).expect("cs is already high");
    let xpt_2046 = Xpt2046::new(spi_device);
    let pen_irq = Input::new(
        peripherals.GPIO34,
        // pull up because active low
        InputConfig::default().with_pull(Pull::Up),
    );
    let touchscreen = Touchscreen::new(xpt_2046, pen_irq, terminal_channel.sender())
        .expect("Failed to initialize the touchscreen");

    spawner.must_spawn(run_touchscreen(touchscreen));

    spawner.must_spawn(update_terminal(terminal_state, terminal));

    let runner = Runner::new(
        pwm_pin,
        runner_channel.receiver(),
        terminal_channel.sender(),
    );

    runner.run().await
}

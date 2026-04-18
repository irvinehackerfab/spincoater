#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Event, Input, InputConfig, Io, Level, Output, OutputConfig, Pull},
    interrupt::software::SoftwareInterruptControl,
    mcpwm::{McPwm, PeripheralClockConfig, operator::PwmPinConfig, timer::PwmWorkingMode},
    spi::master::{Config, Spi},
    time::Rate,
    timer::timg::TimerGroup,
    uart::{self, Uart},
};
use esp_println::println;
use heapless::Vec;
use ibm437::IBM437_9X14_REGULAR;
use mc_esp32::{
    REQUEST_CHANNEL, SECOND_CORE_STACK,
    gpio::{
        display::{
            DISPLAY, ORIENTATION, SPI_BUFFER,
            terminal::{
                TERMINAL,
                channel::{TERMINAL_CHANNEL, TuiEvent},
                update_terminal,
            },
        },
        encoder::ENCODER,
        interrupt_handler,
        pwm::{FREQUENCY, PERIOD, PERIPHERAL_CLOCK_PRESCALER, SETPOINTS},
    },
    motion_profile::{Runner, run},
    rpc::{Context, Dispatcher, FRAME_BUFFER, WIRE_STORAGE},
};
use mipidsi::{interface::SpiInterface, models::ILI9341Rgb565};
use mousefood::{EmbeddedBackend, EmbeddedBackendConfig};
use postcard_rpc::server::{Dispatch, Server, impls::embedded_io_async_v0_6::EioWireSpawn};
use ratatui::Terminal;
use sc_messages::{icd::BAUD_RATE, motion_profile::Setpoint, pwm::STOP_DUTY};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "main is the only place you should be allowed to allocate large buffers."
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // Todo: Replace with defmt
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);
    // Ratatui requires extra memory
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);
    println!("Embassy initialized on the first core!");
    println!("Taking control of the UART port. Please close RTT and open the host PC program.");

    // Setup encoder interrupt to run on the second core
    let software_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        software_interrupts.software_interrupt0,
        software_interrupts.software_interrupt1,
        SECOND_CORE_STACK.take(),
        || {
            // Set the interrupt handler for GPIO.
            // This allows for a slightly lower latency compared to waiting asynchronously.
            let mut io = Io::new(peripherals.IO_MUX);
            io.set_interrupt_handler(interrupt_handler);

            // Initialize encoder pin
            let mut encoder = Input::new(
                peripherals.GPIO27,
                InputConfig::default().with_pull(Pull::Up),
            );
            // Start listening for rising edges
            critical_section::with(|cs| {
                encoder.listen(Event::RisingEdge);
                ENCODER.borrow_ref_mut(cs).replace(encoder);
            });
        },
    );

    // Initialize PWM
    let clock_cfg = PeripheralClockConfig::with_prescaler(PERIPHERAL_CLOCK_PRESCALER);
    let mut mcpwm = McPwm::new(peripherals.MCPWM0, clock_cfg);
    mcpwm.operator0.set_timer(&mcpwm.timer0);
    // connect operator0 to pin IO23:
    // https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#j3
    let mut pwm_pin = mcpwm
        .operator0
        .with_pin_a(peripherals.GPIO26, PwmPinConfig::UP_ACTIVE_HIGH);
    // start timer with timestamp values in the range that we choose.
    let timer_clock_cfg = clock_cfg
        .timer_clock_with_frequency(PERIOD, PwmWorkingMode::Increase, FREQUENCY)
        .expect("Failed to create TimerClockConfig");
    mcpwm.timer0.start(timer_clock_cfg);
    pwm_pin.set_timestamp(*STOP_DUTY);

    // Initialize the display
    // init_with constructs the value in-place to save stack space.
    let terminal = TERMINAL.init_with(|| {
        // init_with constructs the value in-place to save stack space.
        let display = DISPLAY.init_with(|| {
            // https://esp32.implrust.com/tft-display/circuit.html
            let spi = Spi::new(
                peripherals.SPI2,
                Config::default()
                    .with_frequency(Rate::from_mhz(4))
                    .with_mode(esp_hal::spi::Mode::_0),
            )
            .expect("Frequency is within 70kHz..80MHz")
            .into_async()
            // Master In Slave Out. SPI read line from the display to the microcontroller.
            .with_miso(peripherals.GPIO19)
            // Master Out Slave In. This is the SPI data line from the microcontroller to the display. Used to send pixel data and commands.
            .with_mosi(peripherals.GPIO23)
            // Serial Clock. SPI clock signal from the microcontroller. It synchronizes the data being sent.
            .with_sck(peripherals.GPIO18);
            // Chip Select. This tells the display when it should listen to SPI commands. Keep it low (active) when sending data.
            // [`ExclusiveDevice::new_no_delay`] says to have an initial output of high.
            let cs = Output::new(peripherals.GPIO15, Level::High, OutputConfig::default());
            // Data/Command control pin. Set high to send data, low to send commands. Used to switch between writing commands and pixel data.
            let dc = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
            // Resets the display. Useful during startup to make sure the display starts in a known state.
            // Starts off low because [`Ili9341::new`] sets the reset low, then high
            let reset = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());
            let spi_device = ExclusiveDevice::new_no_delay(spi, cs).expect("cs is already high");
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
            ..EmbeddedBackendConfig::default()
        };
        let backend = EmbeddedBackend::new(display, config);
        Terminal::new(backend).expect("Failed to create terminal")
    });

    // Enable the backlight at all times so we can always see the display.
    let _ = Output::new(peripherals.GPIO22, Level::High, OutputConfig::default());

    // Disable touch chip select for now
    let _ = Output::new(peripherals.GPIO33, Level::Low, OutputConfig::default());

    // Initialize vacuum pump pin
    let vacuum_pump_pin = Output::new(peripherals.GPIO17, Level::Low, OutputConfig::default());

    // Setup communication between tasks
    let request_channel = REQUEST_CHANNEL.take();
    let terminal_channel = TERMINAL_CHANNEL.take();
    let to_terminal = terminal_channel.sender();

    // Initialize the setpoint list with a starting setpoint of (0, 0).
    let setpoints = SETPOINTS.init_with(|| Vec::from([Setpoint { rpm: 0, time: 0 }]));

    spawner.must_spawn(update_terminal(terminal, terminal_channel.receiver()));

    // Setup context
    let context = Context::new(request_channel.sender(), vacuum_pump_pin);

    // Setup UART and postcard-rpc after we're done with the spawner
    let config = uart::Config::default().with_baudrate(BAUD_RATE);
    let uart = Uart::new(peripherals.UART0, config)
        .expect("Failed to initialize UART")
        .with_tx(peripherals.GPIO1)
        .with_rx(peripherals.GPIO3)
        .into_async();
    let (rx, tx) = uart.split();
    let dispatcher = Dispatcher::new(context, EioWireSpawn::from(spawner));
    let (wire_rx, wire_tx) = WIRE_STORAGE
        .init(rx, tx)
        .expect("Failed to create wire RX and TX");
    let frame_buffer = FRAME_BUFFER.take();
    let vkk = dispatcher.min_key_len();
    let mut server = Server::new(
        wire_tx,
        wire_rx,
        frame_buffer.as_mut_slice(),
        dispatcher,
        vkk,
    );

    let runner = Runner::new(
        setpoints,
        pwm_pin,
        request_channel.receiver(),
        server.sender(),
    );
    spawner.must_spawn(run(runner));

    loop {
        // Since we lose access to espflash's RTT output as soon as we take control of the UART pins,
        // The only place we can log the error message is to the terminal.
        let err = server.run().await;
        to_terminal.send(TuiEvent::ServerError(err)).await;
    }
}

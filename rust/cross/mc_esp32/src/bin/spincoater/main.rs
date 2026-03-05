#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use bytes::BytesMut;
use embassy_executor::Spawner;
use embassy_net::{StackResources, tcp::TcpSocket};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Receiver, Sender},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Event, Input, InputConfig, Io, Level, Output, OutputConfig, Pull},
    interrupt::software::SoftwareInterruptControl,
    mcpwm::{
        McPwm, PeripheralClockConfig,
        operator::{PwmPin, PwmPinConfig},
        timer::PwmWorkingMode,
    },
    peripherals::MCPWM0,
    rng::Rng,
    spi::master::{Config, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_radio::wifi::{CountryInfo, OperatingClass};
use ibm437::IBM437_9X14_REGULAR;
use mc_esp32::{
    SECOND_CORE_STACK,
    gpio::{
        display::{
            DISPLAY, ORIENTATION, SPI_BUFFER,
            terminal::{
                TERMINAL,
                channel::{
                    ChannelKind, TERMINAL_CHANNEL, TERMINAL_CHANNEL_SIZE, TuiEvent,
                    send_event_or_report,
                },
                update_terminal,
            },
        },
        encoder::{ENCODER, read_rpm},
        interrupt_handler,
        pwm::{FREQUENCY, PERIOD, PERIPHERAL_CLOCK_PRESCALER},
    },
    wifi::{
        AUTH_METHOD, IP_CONFIG, MAX_CONNECTIONS, RADIO, STACK_RESOURCES,
        channel::{HANDLER_CHANNEL_SIZE, RECV_MSG_CHANNEL, SEND_MSG_CHANNEL, send_msg_or_report},
        handle_connections, net_task,
        tcp::{
            BUFFER_SIZE, KEEP_ALIVE, RX_BUFFER, RX_BUFFER_2, TIMEOUT, TX_BUFFER,
            handle_socket_connections,
        },
    },
};
use mipidsi::{interface::SpiInterface, models::ILI9341Rgb565};
use mousefood::{EmbeddedBackend, EmbeddedBackendConfig};
use ratatui::Terminal;
use sc_messages::{Message, STOP_DUTY};

// Wifi requires heap allocation
extern crate alloc;

// Do not hardcode sensitive information like this.
// Instead, pass in the variables as environment variables when you compile, like this:
// SSID=_ PASSWORD=_ cargo run --release
const SSID: &str = env!("SSID");
/// Note: Password must be 8-64 characters.
const PASSWORD: &str = env!("PASSWORD");

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "main is the only place you should be allowed to allocate large buffers."
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);
    // Ratatui requires extra memory
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);
    println!("Embassy initialized on the first core!");

    let radio =
        RADIO.init_with(|| esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller"));
    let wifi_config = esp_radio::wifi::Config::default()
        .with_country_code(CountryInfo::from(*b"US").with_operating_class(OperatingClass::Indoors));
    let (mut wifi_controller, interfaces) =
        esp_radio::wifi::new(radio, peripherals.WIFI, wifi_config)
            .expect("Failed to initialize Wi-Fi controller");
    let net_config = embassy_net::Config::ipv4_static(IP_CONFIG);
    let rng = Rng::new();
    let seed = u64::from(rng.random()) << 32 | u64::from(rng.random());
    // Init network stack
    let (stack, runner) = embassy_net::new(
        interfaces.ap,
        net_config,
        STACK_RESOURCES.init_with(StackResources::new),
        seed,
    );

    // Set the wifi config
    let wifi_config = esp_radio::wifi::ModeConfig::AccessPoint(
        esp_radio::wifi::AccessPointConfig::default()
            .with_ssid(SSID.into())
            .with_auth_method(AUTH_METHOD)
            .with_password(PASSWORD.into())
            .with_max_connections(MAX_CONNECTIONS)
            .with_beacon_timeout(
                u16::try_from(TIMEOUT.as_secs()).expect("10 should fit in a u16."),
            ),
    );
    wifi_controller
        .set_config(&wifi_config)
        .expect("Failed to set Wifi config");

    // Initialize wifi driver
    wifi_controller
        .start_async()
        .await
        .expect("Failed to start wifi");

    // Initialize TCP socket
    let rx_buffer = RX_BUFFER.take();
    let tx_buffer = TX_BUFFER.take();
    let mut socket = TcpSocket::new(stack, rx_buffer, tx_buffer);
    socket.set_timeout(Some(TIMEOUT));
    socket.set_keep_alive(Some(KEEP_ALIVE));
    let buffer = RX_BUFFER_2.init_with(|| BytesMut::with_capacity(BUFFER_SIZE));

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
                peripherals.GPIO17,
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
        .with_pin_a(peripherals.GPIO32, PwmPinConfig::UP_ACTIVE_HIGH);
    // start timer with timestamp values in the range that we choose.
    let timer_clock_cfg = clock_cfg
        .timer_clock_with_frequency(PERIOD, PwmWorkingMode::Increase, FREQUENCY)
        .expect("Failed to create TimerClockConfig");
    mcpwm.timer0.start(timer_clock_cfg);
    pwm_pin.set_timestamp(STOP_DUTY);

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
            let cs = Output::new(peripherals.GPIO16, Level::High, OutputConfig::default());
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

    // Setup communication between tasks
    let recv_msg_channel = RECV_MSG_CHANNEL.take();
    let from_wifi = recv_msg_channel.receiver();
    let send_msg_channel = SEND_MSG_CHANNEL.take();
    let to_transmitter = send_msg_channel.sender();
    let from_msg_handler = send_msg_channel.receiver();
    let terminal_channel = TERMINAL_CHANNEL.take();
    let from_all = terminal_channel.receiver();

    spawner.must_spawn(handle_connections(
        wifi_controller,
        recv_msg_channel.sender(),
        terminal_channel.sender(),
    ));
    spawner.must_spawn(net_task(runner));
    spawner.must_spawn(read_rpm(terminal_channel.sender()));
    spawner.must_spawn(handle_messages(
        pwm_pin,
        from_wifi,
        to_transmitter,
        terminal_channel.sender(),
    ));
    spawner.must_spawn(update_terminal(terminal, from_all));

    // Await connections in a loop.
    handle_socket_connections(
        socket,
        buffer,
        recv_msg_channel.sender(),
        from_msg_handler,
        terminal_channel.sender(),
    )
    .await;
}

/// Handles all control messages.
///
/// This is kept separate from [`mc_esp32::wifi::tcp::receive_unhandled_messages`]
/// in case we ever decide to add other control methods (like the touchscreen).
#[embassy_executor::task]
async fn handle_messages(
    mut pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
    from_wifi: Receiver<'static, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
    to_transmitter: Sender<'static, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
    to_terminal: Sender<'static, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
) {
    loop {
        let message = from_wifi.receive().await;
        match message {
            Message::DutyCycle(duty) => {
                pwm_pin.set_timestamp(duty);
                send_event_or_report(&to_terminal, TuiEvent::DutyChanged(duty)).await;
            }
        }
        send_msg_or_report(&to_transmitter, message, &to_terminal, ChannelKind::SendMsg).await;
    }
}

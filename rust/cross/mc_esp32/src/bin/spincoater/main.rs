#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::sync::atomic::Ordering;

use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::{Ipv4Cidr, StackResources, StaticConfigV4, tcp::TcpSocket};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Receiver, Sender},
};
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Event, Input, InputConfig, Io, Pull},
    interrupt::software::SoftwareInterruptControl,
    mcpwm::{
        McPwm, PeripheralClockConfig,
        operator::{PwmPin, PwmPinConfig},
        timer::PwmWorkingMode,
    },
    peripherals::MCPWM0,
    rng::Rng,
    system::Stack,
    time::Instant,
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_radio::wifi::{CountryInfo, OperatingClass};
use mc_esp32::{
    SECOND_CORE_STACK,
    gpio::{
        encoder::{ENCODER, MOTOR_REVOLUTIONS_DOUBLED},
        interrupt_handler,
        pwm::{FREQUENCY, PERIOD, PERIPHERAL_CLOCK_PRESCALER},
    },
    wifi::{
        AUTH_METHOD, GATEWAY_IP, IP_LISTEN_ENDPOINT, MAX_CONNECTIONS, RADIO, STACK_RESOURCES,
        controller_task, net_task,
        tcp::{
            BUFFER_SIZE, KEEP_ALIVE, RECV_MSG_CHANNEL, RX_BUFFER, SEND_MSG_CHANNEL, TIMEOUT,
            TX_BUFFER, announce_handled_messages, receive_unhandled_messages,
        },
    },
};
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
    // If you ever decide to use COEX (wifi and bluetooth at the same time)
    // then uncomment this line.
    // esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);
    println!("Embassy initialized on the first core!");

    let wifi_config = esp_radio::wifi::Config::default()
        .with_country_code(CountryInfo::from(*b"US").with_operating_class(OperatingClass::Indoors));
    let (mut wifi_controller, interfaces) = esp_radio::wifi::new(
        RADIO.init_with(|| esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller")),
        peripherals.WIFI,
        wifi_config,
    )
    .expect("Failed to initialize Wi-Fi controller");
    println!("Wifi capabilities: {:?}", wifi_controller.capabilities());
    let net_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(GATEWAY_IP, 24),
        gateway: Some(GATEWAY_IP),
        // TODO: I would make the StaticConfigV4 a const, but embassy_net is limited to heapless v0.8.0 so I can't initialize this in a const context until they update.
        dns_servers: Default::default(),
    });
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
            .with_max_connections(MAX_CONNECTIONS),
    );
    wifi_controller
        .set_config(&wifi_config)
        .expect("Failed to set Wifi config");

    // Spawn tasks
    spawner.must_spawn(controller_task(wifi_controller));
    spawner.must_spawn(net_task(runner));

    // Initialize TCP socket
    let rx_buffer = RX_BUFFER.init_with(|| [0u8; BUFFER_SIZE]);
    let tx_buffer = TX_BUFFER.init_with(|| [0u8; BUFFER_SIZE]);
    let mut socket = TcpSocket::new(stack, rx_buffer, tx_buffer);
    socket.set_timeout(Some(TIMEOUT));
    socket.set_keep_alive(Some(KEEP_ALIVE));

    // Setup encoder interrupt to run on the second core
    let software_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        software_interrupts.software_interrupt0,
        software_interrupts.software_interrupt1,
        SECOND_CORE_STACK.init_with(Stack::new),
        || {
            // Set the interrupt handler for GPIO.
            // This allows for a slightly lower latency compared to waiting asynchronously.
            let mut io = Io::new(peripherals.IO_MUX);
            io.set_interrupt_handler(interrupt_handler);

            // Initialize encoder pin
            let mut encoder = Input::new(
                peripherals.GPIO4,
                InputConfig::default().with_pull(Pull::Up),
            );
            // Start listening for rising edges
            critical_section::with(|cs| {
                encoder.listen(Event::RisingEdge);
                ENCODER.borrow_ref_mut(cs).replace(encoder);
            });
        },
    );
    spawner.must_spawn(read_rpm());

    // initialize PWM
    let clock_cfg = PeripheralClockConfig::with_prescaler(PERIPHERAL_CLOCK_PRESCALER);
    let mut mcpwm = McPwm::new(peripherals.MCPWM0, clock_cfg);
    // connect operator0 to timer0
    mcpwm.operator0.set_timer(&mcpwm.timer0);
    // connect operator0 to pin IO23:
    // https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#j3
    let mut pwm_pin = mcpwm
        .operator0
        .with_pin_a(peripherals.GPIO12, PwmPinConfig::UP_ACTIVE_HIGH);
    // start timer with timestamp values in the range that we choose.
    let timer_clock_cfg = clock_cfg
        .timer_clock_with_frequency(PERIOD, PwmWorkingMode::Increase, FREQUENCY)
        .expect("Failed to create TimerClockConfig");
    mcpwm.timer0.start(timer_clock_cfg);
    pwm_pin.set_timestamp(STOP_DUTY);

    // Setup communication between tasks
    let recv_msg_channel = RECV_MSG_CHANNEL.take();
    let mut to_msg_handler = recv_msg_channel.sender();
    let from_receiver = recv_msg_channel.receiver();
    let send_msg_channel = SEND_MSG_CHANNEL.take();
    let to_transmitter = send_msg_channel.sender();
    let mut from_msg_handler = send_msg_channel.receiver();

    spawner.must_spawn(handle_messages(from_receiver, to_transmitter, pwm_pin));

    // Await connections in a loop.
    loop {
        println!("Socket: Waiting for connection...");
        if let Err(err) = socket.accept(IP_LISTEN_ENDPOINT).await {
            println!("Wifi: Accept error: {:?}", err);
            continue;
        }
        println!(
            "Socket: Got connection from address {:?}",
            socket.remote_endpoint()
        );
        let (mut reader, mut writer) = socket.split();
        // Cancel receiving and transmitting as soon as an error occurs.
        // This gives the socket the opportunity to abort.
        match select(
            receive_unhandled_messages(&mut reader, &mut to_msg_handler),
            announce_handled_messages(&mut writer, &mut from_msg_handler),
        )
        .await
        {
            Either::First(err) => {
                println!("Receiver error: {err:?}");
            }
            Either::Second(err) => {
                println!("Transmitter error: {err:?}");
            }
        }
        socket.abort();
        let _ = socket.flush().await;
        if socket.may_recv() {
            // Flush all data from the receive buffer as well.
            let _ = socket.read_with(|bytes| (bytes.len(), ())).await;
        }
    }
}

#[embassy_executor::task]
async fn handle_messages(
    from_receiver: Receiver<'static, NoopRawMutex, Message, 2>,
    to_transmitter: Sender<'static, NoopRawMutex, Message, 2>,
    mut pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
) {
    loop {
        let message = from_receiver.receive().await;
        match message {
            Message::DutyCycle(duty) => pwm_pin.set_timestamp(duty),
        }
        if to_transmitter.try_send(message).is_err() {
            println!(
                "Message handler has no space to send the message. Please consider increasing channel capacity."
            );
            to_transmitter.send(message).await;
        }
    }
}

#[embassy_executor::task]
async fn read_rpm() {
    let mut previous_time = Instant::now();
    loop {
        Timer::after_secs(1).await;
        let motor_revolutions_doubled = MOTOR_REVOLUTIONS_DOUBLED.swap(0, Ordering::Relaxed);
        let time = previous_time.elapsed();
        let time_ms =
            u32::try_from(time.as_millis()).expect("1000 milliseconds should fit in a u32.");
        // (2*motor revolutions) * 1/2 * (20 plate revolutions / 74 motor revolutions) * 1/(`time` ms) * (6000 ms / 1 min)
        // = (2*motor revolutions) * 30,000 / (37 * `time`)
        // Final units: plate revolutions per minute
        let rpm = motor_revolutions_doubled * 30_000 / (37 * (time_ms));
        println!("RPM: {rpm}");
        previous_time += time;
    }
}

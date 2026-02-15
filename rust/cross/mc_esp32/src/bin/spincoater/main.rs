#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_net::{Ipv4Cidr, StackResources, StaticConfigV4, tcp::TcpSocket};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    zerocopy_channel::{Channel, Receiver},
};
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    interrupt::software::SoftwareInterruptControl,
    mcpwm::{
        McPwm, PeripheralClockConfig,
        operator::{PwmPin, PwmPinConfig},
        timer::PwmWorkingMode,
    },
    peripherals::MCPWM0,
    rng::Rng,
    system::Stack,
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_radio::wifi::{CountryInfo, OperatingClass};
use mc_esp32::{
    SECOND_CORE_STACK,
    pwm::{FREQUENCY, MAX_DUTY, PERIPHERAL_CLOCK_PRESCALER, STOP_DUTY},
    wifi::{
        AUTH_METHOD, BUFFER_SIZE, GATEWAY_IP, IP_LISTEN_ENDPOINT, KEEP_ALIVE, MAX_CONNECTIONS,
        MESSAGE_CHANNEL, MESSAGE_CHANNEL_BUFFER, RADIO, RX_BUFFER, STACK_RESOURCES, TIMEOUT,
        TX_BUFFER, controller_task, net_task, recv_message, send_message,
    },
};
use sc_messages::Message;

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

    // Initialize communication between cores
    let msg_channel = MESSAGE_CHANNEL.init_with(|| Channel::new(MESSAGE_CHANNEL_BUFFER.take()));
    let (mut to_msg_handler, mut from_wifi) = msg_channel.split();

    let software_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        software_interrupts.software_interrupt0,
        software_interrupts.software_interrupt1,
        SECOND_CORE_STACK.init_with(Stack::new),
        move || {
            todo!("Start another thread mode executor");
            println!("Embassy initialized on the second core!");

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
                .timer_clock_with_frequency(MAX_DUTY, PwmWorkingMode::Increase, FREQUENCY)
                .expect("Failed to create TimerClockConfig");
            mcpwm.timer0.start(timer_clock_cfg);
            pwm_pin.set_timestamp(STOP_DUTY);
        },
    );

    // Read messages, act on them, and send them back in a loop.
    // This loop is here instead of in a separate embassy task because it allocates too much data onto the stack.
    loop {
        println!("Wifi: Waiting for connection...");
        if let Err(err) = socket.accept(IP_LISTEN_ENDPOINT).await {
            println!("Wifi: Accept error: {:?}", err);
            continue;
        }
        println!("Wifi: Connected to address {:?}", socket.remote_endpoint());
        loop {
            match recv_message(&mut socket).await {
                Ok(message) => {
                    println!("Wifi: Got message: {message:?}");
                    let buffer = to_msg_handler.send().await;
                    *buffer = message;
                    to_msg_handler.send_done();
                    // TODO: Consider putting the write half of the socket in a different task.
                    // Send the message back
                    if let Err(err) = send_message(message, &mut socket).await {
                        break err.handle(&mut socket).await;
                    }
                }
                Err(err) => break err.handle(&mut socket).await,
            }
        }
    }
}

// #[embassy_executor::task]
// async fn handle_messages(
//     mut from_wifi: Receiver<'static, CriticalSectionRawMutex, Message>,
//     mut pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
// ) {
//     loop {
//         let message = from_wifi.receive().await;
//         let message = *message;
//         from_wifi.receive_done();
//         match message {
//             Message::DutyCycle(duty) => pwm_pin.set_timestamp(duty),
//         }
//     }
// }

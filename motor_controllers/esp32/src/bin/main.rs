#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::mcpwm::operator::PwmPinConfig;
use esp_hal::mcpwm::timer::PwmWorkingMode;
use esp_hal::mcpwm::{McPwm, PeripheralClockConfig};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;

extern crate alloc;

// const CONNECTIONS_MAX: usize = 1;
// const L2CAP_CHANNELS_MAX: usize = 1;
const FREQUENCY: Rate = Rate::from_hz(50);
// We can configure this to whatever we like.
// Setting it to 99 allows us to set duty cycle in percentages.
const MAX_DUTY: u16 = 99;
// 5% of max duty
const STOP_DUTY: u16 = 5;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    // generator version: 1.2.0

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // initialize PWM
    let clock_cfg = PeripheralClockConfig::with_frequency(FREQUENCY)
        .expect("Failed to create PeripheralClockConfig");
    let mut mcpwm = McPwm::new(peripherals.MCPWM0, clock_cfg);
    // connect operator0 to timer0
    mcpwm.operator0.set_timer(&mcpwm.timer0);
    // connect operator0 to pin IO23:
    // https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#j3
    let mut pwm_pin = mcpwm
        .operator0
        .with_pin_a(peripherals.GPIO23, PwmPinConfig::UP_ACTIVE_HIGH);
    // start timer with timestamp values in the range that we want.
    let timer_clock_cfg = clock_cfg
        .timer_clock_with_frequency(MAX_DUTY, PwmWorkingMode::Increase, FREQUENCY)
        .expect("Failed to create TimerClockConfig");
    mcpwm.timer0.start(timer_clock_cfg);
    pwm_pin.set_timestamp(STOP_DUTY);

    // esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);
    // COEX needs more RAM - so we've added some more
    // esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    info!("Embassy initialized!");

    // let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    // let (mut _wifi_controller, _interfaces) =
    //     esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
    //         .expect("Failed to initialize Wi-Fi controller");
    // // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    // let transport = BleConnector::new(&radio_init, peripherals.BT, Default::default()).unwrap();
    // let ble_controller = ExternalController::<_, 1>::new(transport);
    // let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
    //     HostResources::new();
    // let _stack = trouble_host::new(ble_controller, &mut resources);

    // TODO: Spawn some tasks

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}

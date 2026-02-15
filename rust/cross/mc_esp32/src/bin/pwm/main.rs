#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    mcpwm::{McPwm, PeripheralClockConfig, operator::PwmPinConfig, timer::PwmWorkingMode},
    timer::timg::TimerGroup,
};
use esp_println::println;
use mc_esp32::pwm::{FREQUENCY, MAX_DUTY, PERIPHERAL_CLOCK_PRESCALER, STOP_DUTY};

extern crate alloc;

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

    println!("Embassy initialized!");

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
    // start timer with timestamp values in the range that we want.
    let timer_clock_cfg = clock_cfg
        .timer_clock_with_frequency(MAX_DUTY, PwmWorkingMode::Increase, FREQUENCY)
        .expect("Failed to create TimerClockConfig");
    println!("Period of the PWM pin: {}", pwm_pin.period());
    mcpwm.timer0.start(timer_clock_cfg);
    pwm_pin.set_timestamp(STOP_DUTY);

    loop {
        Timer::after_secs(1).await;
    }
}

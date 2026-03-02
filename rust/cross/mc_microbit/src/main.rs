#![no_std]
#![no_main]

use {
    defmt::println,
    defmt_rtt as _,
    embassy_nrf::{
        gpio::Output,
        pwm::{DutyCycle, Prescaler, SimpleConfig, SimplePwm},
    },
    embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex},
    embassy_time::Duration,
    microbit_bsp::{
        Button,
        display::{Bitmap, Frame, LedMatrix},
    },
    panic_probe as _,
};

use embassy_executor::Spawner;
use microbit_bsp::Microbit;

type PWMMutex = Mutex<ThreadModeRawMutex, Option<SimplePwm<'static>>>;
static PWM: PWMMutex = Mutex::new(None);

// The frequency of the PWM output in Hz.
// const FREQUENCY: u32 = 50;
/// The microbit allows setting duty cycle values up to 2^15 - 1 (32767).
///
/// With this in mind, a prescaler of 4 results in the highest range of duty cycle values
/// for 50 hz.
const PRESCALER: Prescaler = Prescaler::Div4;
/// This value is derived from [`SimplePwm::set_period`].
///
/// `CLK` = [`embassy_nrf::pwm::PWM_CLK_HZ`] >> [`PRESCALER`] = `1_000_000`
///
/// `max_duty` = `CLK` / `50 Hz` = `20_000`
const PERIOD: u16 = 20_000;
/// The current motor controller reads 5% of [`MAX_DUTY`] as 0% power.
const STOP_DUTY: DutyCycle = DutyCycle::inverted(PERIOD / 20);
// The current motor controller reads 10% of [`PERIOD`] as 100% power.
// const MAX_POWER_DUTY: DutyCycle = DutyCycle::inverted(PERIOD / 10);

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let board = Microbit::default();
    let mut config = SimpleConfig::default();
    config.prescaler = PRESCALER;
    config.max_duty = PERIOD;
    // p0 = P0.02 = ring 0
    // https://tech.microbit.org/hardware/edgeconnector/#pins-and-signals
    let mut pwm = SimplePwm::new_1ch(board.pwm0, board.p0, &config);
    println!("Frequency: {}", pwm.period());
    pwm.set_duty(0, STOP_DUTY);
    pwm.enable();
    // inner scope is so that once the mutex is written to, the MutexGuard is dropped, thus the
    // Mutex is released
    {
        *(PWM.lock().await) = Some(pwm);
    }
    spawner.must_spawn(led(board.display));
    spawner.must_spawn(left_button(board.btn_a, &PWM));
    spawner.must_spawn(right_button(board.btn_b, &PWM));
}

#[embassy_executor::task]
async fn led(mut leds: LedMatrix<Output<'static>, 5, 5>) {
    let bitmap = Bitmap::new(255, 5);
    let matrix = [bitmap, bitmap, bitmap, bitmap, bitmap];
    let frame = Frame::new(matrix);
    loop {
        leds.display(frame, Duration::MAX).await;
    }
}

#[embassy_executor::task]
async fn left_button(mut button: Button, pwm: &'static PWMMutex) {
    loop {
        button.wait_for_falling_edge().await;
        // Stop
        {
            let mut pwm = pwm.lock().await;
            let pwm = pwm.as_mut().expect("PWM should be initialized");
            pwm.set_duty(0, STOP_DUTY);
        }
        println!("Duty: {}", STOP_DUTY);
    }
}

#[embassy_executor::task]
async fn right_button(mut button: Button, pwm: &'static PWMMutex) {
    loop {
        button.wait_for_falling_edge().await;
        // Set PWM to an increasing value
        {
            let mut pwm = pwm.lock().await;
            let pwm = pwm.as_mut().expect("PWM should be initialized");
            let new_duty = pwm.duty(0).value() + 100;
            let new_duty = DutyCycle::inverted(new_duty);
            pwm.set_duty(0, new_duty);
            println!("Duty: {}", new_duty);
        }
    }
}

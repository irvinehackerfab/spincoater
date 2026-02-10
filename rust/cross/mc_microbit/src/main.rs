#![no_std]
#![no_main]

use {
    defmt::println,
    defmt_rtt as _,
    embassy_nrf::{gpio::Output, pwm::SimplePwm},
    embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex},
    embassy_time::Duration,
    microbit_bsp::{
        Button,
        display::{Bitmap, Frame, LedMatrix},
    },
    panic_probe as _,
};

use embassy_nrf::peripherals::PWM0;

use embassy_executor::Spawner;
use microbit_bsp::Microbit;

type PWMMutex = Mutex<ThreadModeRawMutex, Option<SimplePwm<'static, PWM0>>>;
static PWM: PWMMutex = Mutex::new(None);

const FREQUENCY: u32 = 50;
// Note: Update DUTY constants when you change frequency.
// Max duty = pwm.max_duty()
/// 10% of max duty high / 90% of max duty low
// const MAX_THROTTLE_DUTY: u16 = 2_000;
/// 5% of max duty high / 95% of max duty low
const STOP_DUTY: u16 = 19_000;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let board = Microbit::default();
    // p0 = P0.02 = ring 0
    // https://tech.microbit.org/hardware/edgeconnector/#pins-and-signals
    let mut pwm = SimplePwm::new_1ch(board.pwm0, board.p0);
    pwm.set_period(FREQUENCY);
    println!("Max duty: {}", pwm.max_duty());
    // let duty = pwm.max_duty() / 10 * 9;
    // println!("90% of max duty low: {}", duty);
    let duty = pwm.max_duty() / 20 * 19;
    println!("95% of max duty low: {}", duty);
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
            // Subtract b/c we're controlling duty cycle low
            let new_duty = pwm.duty(0) - 100;
            pwm.set_duty(0, new_duty);
            println!("Duty: {}", new_duty);
        }
    }
}

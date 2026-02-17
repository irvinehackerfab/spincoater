use core::{
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};

use critical_section::Mutex;
use embassy_time::Timer;
use esp_hal::{gpio::Input, time::Instant};
use esp_println::println;

/// Provides access to the hall effect sensor.
pub static ENCODER: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

/// The counter for the motor revolutions. This counter is equal to motor revolutions * 2.
pub static MOTOR_REVOLUTIONS_DOUBLED: AtomicU32 = AtomicU32::new(0);

/// Every second, this task prints out the number of plate revolutions per minute if the rpm is not 0.
///
/// # Panics
/// This task panics if more than [`u32::MAX`] milliseconds has passed after 1000 milliseconds has passed,
/// which should never happen.
#[embassy_executor::task]
pub async fn read_rpm() -> ! {
    let mut previous_time = Instant::now();
    loop {
        Timer::after_secs(1).await;
        // Relaxed ordering because the swap does not need to wait for the next atomic increment to the counter.
        let motor_revolutions_doubled = MOTOR_REVOLUTIONS_DOUBLED.swap(0, Ordering::Relaxed);
        let time = previous_time.elapsed();
        if motor_revolutions_doubled != 0 {
            let time_ms =
                u32::try_from(time.as_millis()).expect("1000 milliseconds should fit in a u32.");
            // (2*motor revolutions) * 1/2 * (20 plate revolutions / 74 motor revolutions) * 1/(`time` ms) * (6000 ms / 1 min)
            // = (2*motor revolutions) * 30,000 / (37 * `time`)
            // Final units: plate revolutions per minute
            let rpm = motor_revolutions_doubled * 30_000 / (37 * (time_ms));
            println!("RPM: {rpm}");
        }
        // Increment the time passed regardless of the revolutions counted.
        previous_time += time;
    }
}

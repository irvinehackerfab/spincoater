//! This module contains the functionality for running motion profiles sent by the host PC.
use core::sync::atomic::Ordering;

use crate::{
    LOOP_PERIOD, REQUEST_CHANNEL_LENGTH, REQUEST_RESPONSE_SIGNAL,
    gpio::{
        encoder::MOTOR_REVOLUTIONS_DOUBLED,
        pwm::{SETPOINT_LIST_LENGTH, THROTTLE_CURVE, THROTTLE_POINTS},
    },
    rpc::{HOST_DISCONNECTED, SEQUENCE_NUMBER, WireTx},
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Receiver};
use embassy_time::{Instant, Timer};
use esp_hal::{mcpwm::operator::PwmPin, peripherals::MCPWM0};
use heapless::Vec;
use postcard_rpc::server::Sender;
use sc_messages::{
    icd::MotionProfileStateTopic,
    motion_profile::{self, Request, RequestRefused, Setpoint},
    pwm::{DutyCycle, STOP_DUTY},
};

/// The runner that executes motion profiles.
pub struct Runner {
    setpoints: &'static mut Vec<Setpoint, SETPOINT_LIST_LENGTH>,
    pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
    from_server: Receiver<'static, NoopRawMutex, Request, REQUEST_CHANNEL_LENGTH>,
    to_server: Sender<WireTx>,
}

impl Runner {
    pub fn new(
        setpoints: &'static mut Vec<Setpoint, SETPOINT_LIST_LENGTH>,
        pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
        from_server: Receiver<'static, NoopRawMutex, Request, REQUEST_CHANNEL_LENGTH>,
        to_server: Sender<WireTx>,
    ) -> Self {
        Self {
            setpoints,
            pwm_pin,
            from_server,
            to_server,
        }
    }

    /// Clears all setpoints except for the 0 rpm 0 time element.
    fn clear(&mut self) {
        self.setpoints.truncate(1);
    }

    /// Runs the main control loop.
    pub async fn run(mut self) -> ! {
        loop {
            self.setup().await;
            self.execute_motion_profile().await;
            self.clear();
        }
    }

    /// Sets up the motion profile.
    ///
    /// Repeatedly waits for setpoints until a start message is received.
    async fn setup(&mut self) {
        // We only care if the host disconnects during execution.
        HOST_DISCONNECTED.reset();

        loop {
            match self.from_server.receive().await {
                Request::Add(setpoint) => match self.setpoints.push(setpoint.clone()) {
                    Ok(()) => REQUEST_RESPONSE_SIGNAL.signal(Ok(())),
                    Err(_) => REQUEST_RESPONSE_SIGNAL.signal(Err(RequestRefused::TooManySetpoints)),
                },
                Request::ClearSetpoints => {
                    self.clear();
                    REQUEST_RESPONSE_SIGNAL.signal(Ok(()));
                }
                Request::Start => {
                    REQUEST_RESPONSE_SIGNAL.signal(Ok(()));
                    // `postcard_rpc` sometimes sends setpoints out of order, so we have to sort them.
                    self.setpoints.sort_unstable();
                    break;
                }
                Request::Stop => {
                    REQUEST_RESPONSE_SIGNAL.signal(Err(RequestRefused::NotRunning));
                }
            }
        }
        // Since we reset the time, we must reset the motor revolutions counter as well.
        MOTOR_REVOLUTIONS_DOUBLED.store(0, Ordering::Relaxed);
    }

    /// Executes the motion profile,
    /// logging info every iteration and checking for a stop command.
    async fn execute_motion_profile(&mut self) {
        let starting_time = Instant::now();
        let mut previous_sleep_end = starting_time;
        let mut setpoint_idx = 0;
        loop {
            // Sleep must be called at the start so LOOP_PERIOD time can pass before the current rpm is calculated.
            previous_sleep_end = Self::sleep(previous_sleep_end).await;

            // Check for stop requests.
            if let Ok(command) = self.from_server.try_receive() {
                match command {
                    Request::Add(_) | Request::ClearSetpoints | Request::Start => {
                        REQUEST_RESPONSE_SIGNAL.signal(Err(RequestRefused::Running));
                    }
                    Request::Stop => {
                        REQUEST_RESPONSE_SIGNAL.signal(Ok(()));
                        self.pwm_pin.set_timestamp(*STOP_DUTY);
                        let _ = self
                            .to_server
                            .log_str("Motion profile stopped early.")
                            .await;
                        break;
                    }
                }
            }

            // Check for host disconnects.
            if HOST_DISCONNECTED.try_take().is_some() {
                self.pwm_pin.set_timestamp(*STOP_DUTY);
                break;
            }

            let elapsed_since_start_micros = starting_time.elapsed().as_micros();
            let Some((setpoint_rpm, setpoint_duty_cycle)) = self
                .feedforward(&mut setpoint_idx, elapsed_since_start_micros)
                .await
            else {
                break;
            };
            self.pwm_pin.set_timestamp(*setpoint_duty_cycle);

            // todo!("Add feedback")
            // This is where we would add feedback.
            let current_rpm = Self::calculate_rpm();

            // Logging
            let state = Some(motion_profile::State {
                setpoint_rpm,
                current_rpm,
                duty_cycle: setpoint_duty_cycle,
                time: elapsed_since_start_micros,
            });
            if self
                .to_server
                .publish::<MotionProfileStateTopic>(SEQUENCE_NUMBER, &state)
                .await
                .is_err()
            {
                // The host PC disconnected, so we need to stop.
                self.pwm_pin.set_timestamp(*STOP_DUTY);
                break;
            }
        }
        // Report that there is no more state.
        let _ = self
            .to_server
            .publish::<MotionProfileStateTopic>(SEQUENCE_NUMBER, &None)
            .await;
    }

    /// Sleeps if less than [`LOOP_PERIOD`] time has passed since the last end of this function.
    ///
    /// Returns the instant at the end of this function call.
    /// This value should be passed to the next call to this method.
    async fn sleep(previous_sleep_end: Instant) -> Instant {
        let elapsed_since_previous_sleep_end = previous_sleep_end.elapsed();
        // Only sleep if less than LOOP_PERIOD time has passed since the previous loop start.
        match LOOP_PERIOD.checked_sub(elapsed_since_previous_sleep_end) {
            Some(time_to_sleep) => {
                let before_sleep = Instant::now();
                Timer::after(time_to_sleep).await;
                // Manually calculating the end of the function makes this function immune to oversleep from the timer.
                before_sleep
                    .checked_add(time_to_sleep)
                    .expect("This program will not run for 584558 years.")
            }
            None => Instant::now(),
        }
    }

    /// Calculates the setpoint rpm and duty cycle for this timestep.
    ///
    /// If there are no more setpoints to use, the method will disable PWM, log that the motion profile finished, and return [`None`].
    ///
    /// If the rpm doesn't fit in a [`u16`], the method will disable PWM, log the error, and then return [`None`].
    ///
    /// It must disable PWM itself because it awaits upon failure,
    /// and we don't want to wait on some other task before disabling PWM.
    async fn feedforward(
        &mut self,
        setpoint_idx: &mut usize,
        elapsed_since_start_micros: u64,
    ) -> Option<(u16, DutyCycle)> {
        // Get next pair of setpoints.
        let Some((previous_setpoint, current_setpoint)) =
            self.next_setpoint_pair(setpoint_idx, elapsed_since_start_micros)
        else {
            self.pwm_pin.set_timestamp(*STOP_DUTY);
            let _ = self.to_server.log_str("Motion profile done.").await;
            return None;
        };

        // Get setpoint rpm.
        let Some(setpoint_rpm) = self
            .next_setpoint_rpm(
                previous_setpoint,
                current_setpoint,
                elapsed_since_start_micros,
            )
            .await
        else {
            self.pwm_pin.set_timestamp(*STOP_DUTY);
            let _ = self
                .to_server
                .log_str("Failed to calculate setpoint RPM. Stopping!")
                .await;
            return None;
        };
        // Then we need to linearly interpolate to find the required duty cycle.
        Some((setpoint_rpm, linear_interpolation(setpoint_rpm)))
    }

    /// Gets the next pair of setpoints.
    ///
    /// Returns [`None`] if there are no more pairs of setpoints to act on.
    fn next_setpoint_pair(
        &self,
        setpoint_idx: &mut usize,
        elapsed_since_start_micros: u64,
    ) -> Option<(&Setpoint, &Setpoint)> {
        loop {
            let next_setpoint_idx = setpoint_idx.checked_add(1)?;
            match (
                self.setpoints.get(*setpoint_idx),
                self.setpoints.get(next_setpoint_idx),
            ) {
                (Some(previous_setpoint), Some(current_setpoint)) => {
                    // Only act on setpoints that haven't passed.
                    if elapsed_since_start_micros <= current_setpoint.time {
                        return Some((previous_setpoint, current_setpoint));
                    }
                    *setpoint_idx = next_setpoint_idx;
                }
                // The motion profile is done.
                (_, None) | (None, _) => return None,
            }
        }
    }

    /// Gets the next setpoint rpm.
    ///
    /// See [Wikipedia's explanation for linear approximation](https://en.wikipedia.org/wiki/Linear_interpolation#Linear_interpolation_as_an_approximation).
    ///
    /// # Errors
    /// Returns None if one of multiple possible arithmetic errors occurs.
    async fn next_setpoint_rpm(
        &self,
        previous_setpoint: &Setpoint,
        current_setpoint: &Setpoint,
        elapsed_since_start_micros: u64,
    ) -> Option<u16> {
        // We need to increase the size of some numbers to prevent overflow.
        let previous_setpoint_rpm = u64::from(previous_setpoint.rpm);
        let current_setpoint_rpm = u64::from(current_setpoint.rpm);
        let delta_rpm = current_setpoint_rpm.saturating_sub(previous_setpoint_rpm);
        let delta_time = elapsed_since_start_micros.saturating_sub(previous_setpoint.time);
        let Some(numerator) = delta_rpm.checked_mul(delta_time) else {
            let _ = self.to_server.log_str("Multiplication overflowed!").await;
            return None;
        };
        let denominator = current_setpoint.time.saturating_sub(previous_setpoint.time);
        let Some(interpolation) = numerator.checked_div(denominator) else {
            return Some(previous_setpoint.rpm);
        };
        let Ok(interpolation) = u16::try_from(interpolation) else {
            let _ = self
                .to_server
                .log_str("Interpolation exceeds u16::MAX!")
                .await;
            return None;
        };
        let Some(result) = previous_setpoint.rpm.checked_add(interpolation) else {
            let _ = self.to_server.log_str("RPM exceeds u16::MAX!").await;
            return None;
        };
        Some(result)
    }

    /// Calculates the current rpm using [`LOOP_PERIOD`] as the amount of time that has passed.
    ///
    /// This function never fails. If the RPM is greater than [`u16::MAX`], [`u16::MAX`] is returned.
    #[allow(clippy::cast_possible_truncation)]
    fn calculate_rpm() -> u16 {
        // Relaxed ordering because the order of instructions does not matter for the swap.
        let motor_revolutions_doubled =
            u64::from(MOTOR_REVOLUTIONS_DOUBLED.swap(0, Ordering::Relaxed));
        let time_micros = LOOP_PERIOD.as_micros();
        // (2*motor revolutions) * 1/2 * 1/(`time` μs) * (10^6 μs / 1 s) * (60 s / 1 min)
        // = (2*motor revolutions) * 30,000,000 / `time`
        // Final units: motor revolutions per minute
        // Note: This multiplication is saturating because there's absolutely no chance for it to exceed 2^64 - 1.
        let numerator = motor_revolutions_doubled.saturating_mul(30_000_000);
        let Some(rpm) = numerator.checked_div(time_micros) else {
            return 0;
        };
        rpm as u16
    }
}

/// Performs linear interpolation on [`THROTTLE_CURVE`] to find the setpoint duty cycle.
///
/// [`THROTTLE_CURVE`] must be nonzero and [`THROTTLE_CURVE`]`[0]` must have increasing values.
///
/// This function never fails. If the duty cycle is greater than [`u16::MAX`], [`u16::MAX`] is returned.
///
/// # Implementation
/// See [Wikipedia](https://en.wikipedia.org/wiki/Linear_interpolation#Linear_interpolation_as_an_approximation) for more info.
#[allow(clippy::cast_possible_truncation)]
fn linear_interpolation(setpoint_rpm: u16) -> DutyCycle {
    // Everything here is in u32 to prevent overflow.
    let setpoint_rpm = u32::from(setpoint_rpm);
    if setpoint_rpm <= THROTTLE_CURVE[0][0] {
        return THROTTLE_CURVE[1][0].into();
    }
    for ((rpm_0, rpm_1), (duty_0, duty_1)) in THROTTLE_CURVE[0]
        .iter()
        .zip(&THROTTLE_CURVE[0][1..THROTTLE_POINTS])
        .zip(
            THROTTLE_CURVE[1]
                .iter()
                .zip(&THROTTLE_CURVE[1][1..THROTTLE_POINTS]),
        )
    {
        if setpoint_rpm >= *rpm_0 {
            let delta_duty = duty_1.saturating_sub(*duty_0);
            let delta_rpm = setpoint_rpm.saturating_sub(*rpm_0);
            // This multiplication is saturating because there is no chance for the product to exceed 2^32 - 1.
            let numerator = delta_duty.saturating_mul(delta_rpm);
            let denominator = rpm_1.saturating_sub(*rpm_0);
            let Some(result) = numerator.checked_div(denominator) else {
                return (*duty_0).into();
            };
            let result = duty_0.saturating_add(result);
            return result.into();
        }
    }
    // The setpoint rpm is higher than any known rpm, so just return the highest rpm.
    THROTTLE_CURVE[1][THROTTLE_POINTS - 1].into()
}

/// Runs the [`Runner`] forever.
#[embassy_executor::task]
pub async fn run(runner: Runner) {
    runner.run().await;
}

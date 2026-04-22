//! This module contains the functionality for running motion profiles sent by the host PC.
use core::{num::TryFromIntError, sync::atomic::Ordering};

use crate::{
    LOOP_PERIOD, REQUEST_CHANNEL_LENGTH, REQUEST_RESPONSE_SIGNAL,
    gpio::{
        encoder::MOTOR_REVOLUTIONS_DOUBLED,
        pwm::{SETPOINT_LIST_LENGTH, THROTTLE_CURVE, THROTTLE_POINTS},
    },
    rpc::{SEQUENCE_NUMBER, WireTx},
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Receiver};
use embassy_time::{Duration, Instant, Timer};
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
                    break;
                }
                Request::Stop => {
                    REQUEST_RESPONSE_SIGNAL.signal(Err(RequestRefused::NotRunning));
                }
            }
        }
        // Change the setpoints from time since last setpoint to time since the start of the motion profile.
        // We could potentially log the return value of this (AKA the total motion profile time)
        // for something like a progress bar.
        self.setpoints.iter_mut().fold(0u64, |time, setpoint| {
            let delta_time = setpoint.time;
            setpoint.time += time;
            time + delta_time
        });
    }

    /// Executes the motion profile,
    /// logging info every iteration and checking for a stop command.
    async fn execute_motion_profile(&mut self) {
        let starting_time = Instant::now();
        let mut previous_iteration = starting_time;
        // Since we reset the time, we must reset the motor revolutions counter as well.
        MOTOR_REVOLUTIONS_DOUBLED.store(0, Ordering::Relaxed);
        let mut setpoint_idx = 0;
        loop {
            // This is prone to accumulating oversleep, but that's not important here.
            Timer::after(LOOP_PERIOD).await;
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
                        return;
                    }
                }
            }
            let elapsed_since_start_micros = starting_time.elapsed().as_micros();
            let Some((setpoint_rpm, setpoint_duty_cycle)) = self
                .feedforward(&mut setpoint_idx, elapsed_since_start_micros)
                .await
            else {
                return;
            };
            self.pwm_pin.set_timestamp(setpoint_duty_cycle);

            // todo!("Add feedback")
            // This is where we would add feedback.
            let elapsed_since_last_iteration = previous_iteration.elapsed();
            let current_rpm = match Self::calculate_rpm(elapsed_since_last_iteration) {
                Ok(rpm) => rpm,
                Err(err) => {
                    self.pwm_pin.set_timestamp(*STOP_DUTY);
                    let _ = self
                        .to_server
                        .log_fmt(format_args!("Failed to calculate rpm: {err}. Stopping!"))
                        .await;
                    return;
                }
            };

            // Logging
            let duty_cycle = match DutyCycle::try_from(setpoint_duty_cycle) {
                Ok(duty_cycle) => duty_cycle,
                Err(err) => {
                    self.pwm_pin.set_timestamp(*STOP_DUTY);
                    let _ = self
                        .to_server
                        .log_fmt(format_args!(
                            "Failed to create duty cycle: {err}. Stopping!"
                        ))
                        .await;
                    return;
                }
            };
            let state = motion_profile::State {
                setpoint_rpm,
                current_rpm,
                duty_cycle,
                time: elapsed_since_start_micros,
            };
            if self
                .to_server
                .publish::<MotionProfileStateTopic>(SEQUENCE_NUMBER, &state)
                .await
                .is_err()
            {
                // The host PC disconnected, so we need to stop.
                self.pwm_pin.set_timestamp(*STOP_DUTY);
                return;
            }
            // Increment time
            previous_iteration += elapsed_since_last_iteration;
        }
    }

    /// Calculates the setpoint rpm and duty cycle for this timestep.
    ///
    /// If there are no more setpoints to use, the method will disable PWM, log that the motion profile finished, and return [`None`].
    ///
    /// If the rpm doesn't fit in a [`u16`], the method will disable PWM, log the error, and then return [`None`].
    async fn feedforward(
        &mut self,
        setpoint_idx: &mut usize,
        elapsed_since_start_micros: u64,
    ) -> Option<(u16, u16)> {
        // Get next pair of setpoints.
        let Some((previous_setpoint, current_setpoint)) =
            self.next_setpoint_pair(setpoint_idx, elapsed_since_start_micros)
        else {
            self.pwm_pin.set_timestamp(*STOP_DUTY);
            let _ = self.to_server.log_str("Motion profile done.").await;
            return None;
        };
        // Get setpoint rpm.
        let setpoint_rpm = match Self::next_setpoint_rpm(
            previous_setpoint,
            current_setpoint,
            elapsed_since_start_micros,
        ) {
            Ok(rpm) => rpm,
            Err(err) => {
                self.pwm_pin.set_timestamp(*STOP_DUTY);
                let _ = self
                    .to_server
                    .log_fmt(format_args!(
                        "Failed to calculate setpoint RPM: {err}. Stopping!"
                    ))
                    .await;
                return None;
            }
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
            match (
                self.setpoints.get(*setpoint_idx),
                self.setpoints.get(*setpoint_idx + 1),
            ) {
                (Some(previous_setpoint), Some(current_setpoint)) => {
                    // Only act on setpoints that haven't passed.
                    if elapsed_since_start_micros <= current_setpoint.time {
                        return Some((previous_setpoint, current_setpoint));
                    }
                    *setpoint_idx += 1;
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
    /// Returns an error if the rpm cannot fit in a [`u16`].
    fn next_setpoint_rpm(
        previous_setpoint: &Setpoint,
        current_setpoint: &Setpoint,
        elapsed_since_start_micros: u64,
    ) -> Result<u16, TryFromIntError> {
        // We need to increase the size of some numbers to prevent overflow.
        let previous_setpoint_rpm = u64::from(previous_setpoint.rpm);
        let current_setpoint_rpm = u64::from(current_setpoint.rpm);
        let delta_rpm = current_setpoint_rpm - previous_setpoint_rpm;
        let delta_time = elapsed_since_start_micros - previous_setpoint.time;
        let numerator = delta_rpm * delta_time;
        let denominator = current_setpoint.time - previous_setpoint.time;
        u16::try_from(previous_setpoint_rpm + numerator / denominator)
    }

    /// Calculates the current rpm.
    ///
    /// # Errors
    /// Returns an error if time can't fit in a [`u32`],
    /// or RPM can't fit in a [`u16`].
    fn calculate_rpm(elapsed_since_last_iteration: Duration) -> Result<u16, TryFromIntError> {
        // Relaxed ordering because the order of instructions does not matter for the swap.
        let motor_revolutions_doubled = MOTOR_REVOLUTIONS_DOUBLED.swap(0, Ordering::Relaxed);
        let time_ms = u32::try_from(elapsed_since_last_iteration.as_millis())?;
        // Avoid dividing by zero
        if time_ms == 0 {
            return Ok(0);
        }
        // (2*motor revolutions) * 1/2 * (20 plate revolutions / 74 motor revolutions) * 1/(`time` ms) * (6000 ms / 1 min)
        // = (2*motor revolutions) * 30,000 / (37 * `time`)
        // Final units: plate revolutions per minute
        let rpm = motor_revolutions_doubled * 30_000 / (37 * (time_ms));
        u16::try_from(rpm)
    }
}

/// Performs linear interpolation on [`THROTTLE_CURVE`] to find the setpoint duty cycle.
///
/// [`THROTTLE_CURVE`] must be nonzero and [`THROTTLE_CURVE`]`[0]` must have increasing values.
///
/// [Wikipedia explanation](https://en.wikipedia.org/wiki/Linear_interpolation#Linear_interpolation_as_an_approximation)
fn linear_interpolation(setpoint_rpm: u16) -> u16 {
    if setpoint_rpm <= THROTTLE_CURVE[0][0] {
        return THROTTLE_CURVE[1][0];
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
        if setpoint_rpm == *rpm_0 {
            return *duty_0;
        }
        if setpoint_rpm > *rpm_0 {
            let delta_duty = u32::from(duty_1 - duty_0);
            let delta_rpm = u32::from(setpoint_rpm - rpm_0);
            let numerator = delta_duty * delta_rpm;
            let denominator = u32::from(rpm_1 - rpm_0);
            let result =
                u16::try_from(numerator / denominator).expect("The rpm should never exceed 65535.");
            return duty_0 + result;
        }
    }
    THROTTLE_CURVE[1][THROTTLE_POINTS - 1]
}

/// Runs the [`Runner`] forever.
#[embassy_executor::task]
pub async fn run(runner: Runner) {
    runner.run().await;
}

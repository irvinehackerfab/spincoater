//! This module contains the functionality for running motion profiles sent by the host PC.

use crate::{
    LOOP_PERIOD, REQUEST_CHANNEL_LENGTH,
    gpio::{
        encoder::{ENCODER_STATE, EncoderState, calculate_rpm},
        pwm::{SETPOINT_LIST_LENGTH, STATIC_DUTY, linear_conversion},
    },
    pid::{error, next_control_output},
    rpc::{HOST_DISCONNECTED, SEQUENCE_NUMBER, WireTx},
    runners::sleep,
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Receiver, signal::Signal};
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
    server_request_responder: &'static Signal<NoopRawMutex, Result<(), RequestRefused>>,
}

impl Runner {
    pub fn new(
        setpoints: &'static mut Vec<Setpoint, SETPOINT_LIST_LENGTH>,
        pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
        from_server: Receiver<'static, NoopRawMutex, Request, REQUEST_CHANNEL_LENGTH>,
        to_server: Sender<WireTx>,
        server_request_responder: &'static Signal<NoopRawMutex, Result<(), RequestRefused>>,
    ) -> Self {
        Self {
            setpoints,
            pwm_pin,
            from_server,
            to_server,
            server_request_responder,
        }
    }

    /// Clears all setpoints except for the 0 rpm 0 time element.
    fn clear(&mut self) {
        self.setpoints.truncate(1);
    }

    /// Runs the main control loop.
    async fn run(mut self) -> ! {
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
                    Ok(()) => self.server_request_responder.signal(Ok(())),
                    Err(_) => self
                        .server_request_responder
                        .signal(Err(RequestRefused::TooManySetpoints)),
                },
                Request::ClearSetpoints => {
                    self.clear();
                    self.server_request_responder.signal(Ok(()));
                }
                Request::Start => {
                    self.server_request_responder.signal(Ok(()));
                    // `postcard_rpc` sometimes sends setpoints out of order, so we have to sort them.
                    self.setpoints.sort();
                    break;
                }
                Request::Stop => {
                    self.server_request_responder
                        .signal(Err(RequestRefused::NotRunning));
                }
            }
        }
        // Overcome static friction.
        self.pwm_pin.set_timestamp(STATIC_DUTY);
        Timer::after(LOOP_PERIOD).await;
        // Since we are starting again, we must reset the encoder state.
        ENCODER_STATE.with(EncoderState::reset);
    }

    /// Executes the motion profile,
    /// logging info every iteration and checking for a stop command.
    async fn execute_motion_profile(&mut self) {
        let starting_time = Instant::now();
        let mut previous_sleep_end = starting_time;
        let mut setpoint_idx = 0;
        loop {
            // Sleep must be called at the start so LOOP_PERIOD time can pass before the current rpm is calculated.
            previous_sleep_end = sleep(previous_sleep_end).await;

            // Check for stop requests.
            if let Ok(command) = self.from_server.try_receive() {
                match command {
                    Request::Add(_) | Request::ClearSetpoints | Request::Start => {
                        self.server_request_responder
                            .signal(Err(RequestRefused::Running));
                    }
                    Request::Stop => {
                        self.server_request_responder.signal(Ok(()));
                        self.pwm_pin.set_timestamp(STOP_DUTY);
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
                self.pwm_pin.set_timestamp(STOP_DUTY);
                break;
            }

            let elapsed_since_start_micros = starting_time.elapsed().as_micros();

            // Feedforward
            let Some((setpoint_rpm, setpoint_duty_cycle)) = self
                .feedforward(&mut setpoint_idx, elapsed_since_start_micros)
                .await
            else {
                break;
            };

            // Feedback
            let current_rpm = calculate_rpm();
            let rpm_error = error(setpoint_rpm, current_rpm);
            let output = next_control_output(rpm_error);
            let duty_cycle = (*setpoint_duty_cycle).saturating_add_signed(output);

            self.pwm_pin.set_timestamp(duty_cycle);

            // Logging
            let state = Some(motion_profile::State {
                setpoint_rpm,
                current_rpm,
                rpm_error,
                duty_cycle: DutyCycle::from(duty_cycle),
                time: elapsed_since_start_micros,
            });
            if self
                .to_server
                .publish::<MotionProfileStateTopic>(SEQUENCE_NUMBER, &state)
                .await
                .is_err()
            {
                // The host PC disconnected, so we need to stop.
                self.pwm_pin.set_timestamp(STOP_DUTY);
                break;
            }
        }
        // Report that there is no more state.
        let _ = self
            .to_server
            .publish::<MotionProfileStateTopic>(SEQUENCE_NUMBER, &None)
            .await;
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
            self.pwm_pin.set_timestamp(STOP_DUTY);
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
            self.pwm_pin.set_timestamp(STOP_DUTY);
            let _ = self
                .to_server
                .log_str("Failed to calculate setpoint RPM. Stopping!")
                .await;
            return None;
        };
        // Then we need to linearly interpolate to find the required duty cycle.
        Some((setpoint_rpm, linear_conversion(setpoint_rpm)))
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
}

/// Runs the [`Runner`] forever.
#[embassy_executor::task]
pub async fn run(runner: Runner) {
    runner.run().await;
}

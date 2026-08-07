//! This module contains the functionality for running motion profiles sent by the host PC.

use crate::{
    RunnerRequestReceiver, RunnerResponseSenderMutex,
    gpio::{
        encoder::{ENCODER, ENCODER_STATE, EncoderState, calculate_average_rpm},
        pwm::{SETPOINT_LIST_LENGTH, linear_conversion},
    },
    pid::{neg_error, next_control_output},
    runners::sleep,
};
use embassy_time::Instant;
use esp_hal::{gpio::Event, mcpwm::operator::PwmPin, peripherals::MCPWM0};
use heapless::Vec;
use sc_messages::{
    icd,
    motion_profile::{HostMessage, McuMessage, Setpoint, State},
    pwm::{DutyCycle, HALF_POWER_DUTY, STOP_DUTY},
};

/// The runner that executes motion profiles.
pub struct Runner {
    setpoints: &'static mut Vec<Setpoint, SETPOINT_LIST_LENGTH>,
    pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
    from_server: RunnerRequestReceiver,
    to_server: &'static RunnerResponseSenderMutex,
}

impl Runner {
    pub fn new(
        setpoints: &'static mut Vec<Setpoint, SETPOINT_LIST_LENGTH>,
        pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
        from_server: RunnerRequestReceiver,
        to_server: &'static RunnerResponseSenderMutex,
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
    async fn run(mut self) -> ! {
        loop {
            if let Some(setpoint) = self.setup().await {
                // Since we are starting again, we must reset the encoder state.
                ENCODER_STATE.with(EncoderState::reset);
                // Start listening for interrupts
                ENCODER.with(|encoder| {
                    encoder
                        .as_mut()
                        .expect("The runner cannot function without the encoder.")
                        .listen(Event::RisingEdge);
                });
                self.execute_single_rpm(&setpoint).await;
            } else {
                // Since we are starting again, we must reset the encoder state.
                ENCODER_STATE.with(EncoderState::reset);
                // Start listening for interrupts
                ENCODER.with(|encoder| {
                    encoder
                        .as_mut()
                        .expect("The runner cannot function without the encoder.")
                        .listen(Event::RisingEdge);
                });
                self.execute_motion_profile().await;
            }
            // Stop listening for interrupts
            ENCODER.with(|encoder| {
                encoder
                    .as_mut()
                    .expect("The runner cannot function without the encoder.")
                    .unlisten();
            });
            self.clear();
        }
    }

    /// Sets up the runner using commands from the server.
    ///
    /// Repeatedly waits for setpoints until a start message is received.
    ///
    /// Returns a setpoint if the server asked to run at a single RPM value.
    #[must_use]
    async fn setup(&mut self) -> Option<Setpoint> {
        loop {
            match self.from_server.receive().await {
                HostMessage::Add(setpoint) => {
                    let _ = self.setpoints.push(setpoint.clone());
                }
                HostMessage::ClearSetpoints => self.clear(),
                HostMessage::Start => {
                    self.from_server.receive_done();
                    // Sort the setpoints just in case the host PC sent them out of order.
                    self.setpoints.sort();
                    return None;
                }
                HostMessage::Run(setpoint) => {
                    let setpoint = setpoint.clone();
                    self.from_server.receive_done();
                    return Some(setpoint);
                }
                HostMessage::Stop => {}
            }
            self.from_server.receive_done();
        }
    }

    /// Executes a single RPM,
    /// logging info every iteration and checking for a stop command.
    async fn execute_single_rpm(&mut self, setpoint: &Setpoint) {
        let starting_time = Instant::now();
        let mut previous_sleep_end = starting_time;
        // Feedforward
        // We can get the feedforward for the entire run.
        let setpoint_duty_cycle = linear_conversion(setpoint.rpm);

        loop {
            // Sleep must be called at the start so LOOP_PERIOD time can pass before the current rpm is calculated.
            previous_sleep_end = sleep(previous_sleep_end).await;

            // Check for stop requests.
            if let Some(message) = self.from_server.try_receive() {
                let should_stop = matches!(message, HostMessage::Stop);
                self.from_server.receive_done();
                if should_stop {
                    break;
                }
            }

            // Check if we finished.
            let time_since_start_micros = starting_time.elapsed().as_micros();
            if time_since_start_micros >= setpoint.time {
                break;
            }

            // Feedback
            let current_rpm =
                ENCODER_STATE.with(|state| calculate_average_rpm(&state.rpm_ring_buffer));
            let negative_rpm_error = neg_error(setpoint.rpm, current_rpm);
            let output = next_control_output(negative_rpm_error);
            let duty_cycle = (*setpoint_duty_cycle)
                .saturating_add_signed(output)
                .clamp(STOP_DUTY, HALF_POWER_DUTY);

            self.pwm_pin.set_timestamp(duty_cycle);

            // Logging
            let state = State {
                setpoint_rpm: setpoint.rpm,
                current_rpm,
                rpm_error: negative_rpm_error.saturating_neg(),
                duty_cycle: DutyCycle::from(duty_cycle),
                time: time_since_start_micros,
            };
            self.send_message(&McuMessage::State(state)).await;
        }
        // Disable PWM
        self.pwm_pin.set_timestamp(STOP_DUTY);
        // Report that the motion profile is finished.
        self.send_message(&McuMessage::Finished).await;
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
            if let Some(message) = self.from_server.try_receive() {
                let should_stop = matches!(message, HostMessage::Stop);
                self.from_server.receive_done();
                if should_stop {
                    break;
                }
            }

            let elapsed_since_start_micros = starting_time.elapsed().as_micros();

            // Feedforward
            let Some((setpoint_rpm, setpoint_duty_cycle)) =
                self.feedforward(&mut setpoint_idx, elapsed_since_start_micros)
            else {
                break;
            };

            // Feedback
            let current_rpm =
                ENCODER_STATE.with(|state| calculate_average_rpm(&state.rpm_ring_buffer));
            let rpm_error = neg_error(setpoint_rpm, current_rpm);
            let output = next_control_output(rpm_error);
            let duty_cycle = (*setpoint_duty_cycle)
                .saturating_add_signed(output)
                .clamp(STOP_DUTY, HALF_POWER_DUTY);

            self.pwm_pin.set_timestamp(duty_cycle);

            // Logging
            let state = State {
                setpoint_rpm,
                current_rpm,
                rpm_error,
                duty_cycle: DutyCycle::from(duty_cycle),
                time: elapsed_since_start_micros,
            };
            self.send_message(&McuMessage::State(state)).await;
        }
        // Disable PWM
        self.pwm_pin.set_timestamp(STOP_DUTY);
        // Report that the motion profile is finished.
        self.send_message(&McuMessage::Finished).await;
    }

    /// Calculates the setpoint rpm and duty cycle for this timestep.
    ///
    /// If there are no more setpoints to use, the method will disable PWM, log that the motion profile finished, and return [`None`].
    ///
    /// If the rpm doesn't fit in a [`u16`], the method will disable PWM, log the error, and then return [`None`].
    fn feedforward(
        &mut self,
        setpoint_idx: &mut usize,
        elapsed_since_start_micros: u64,
    ) -> Option<(u16, DutyCycle)> {
        // Get next pair of setpoints.
        let (previous_setpoint, current_setpoint) =
            self.next_setpoint_pair(setpoint_idx, elapsed_since_start_micros)?;

        // Get setpoint rpm.
        let setpoint_rpm = Self::next_setpoint_rpm(
            previous_setpoint,
            current_setpoint,
            elapsed_since_start_micros,
        )?;
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
    /// Returns [`None`] if one of multiple possible arithmetic errors occurs.
    fn next_setpoint_rpm(
        previous_setpoint: &Setpoint,
        current_setpoint: &Setpoint,
        elapsed_since_start_micros: u64,
    ) -> Option<u16> {
        // We need to increase the size of some numbers to prevent overflow.
        let previous_setpoint_rpm = u64::from(previous_setpoint.rpm);
        let current_setpoint_rpm = u64::from(current_setpoint.rpm);
        let delta_rpm = current_setpoint_rpm.saturating_sub(previous_setpoint_rpm);
        let delta_time = elapsed_since_start_micros.saturating_sub(previous_setpoint.time);
        let numerator = delta_rpm.checked_mul(delta_time)?;
        let denominator = current_setpoint.time.saturating_sub(previous_setpoint.time);
        let Some(interpolation) = numerator.checked_div(denominator) else {
            return Some(previous_setpoint.rpm);
        };
        let Ok(interpolation) = u16::try_from(interpolation) else {
            return None;
        };
        let result = previous_setpoint.rpm.checked_add(interpolation)?;
        Some(result)
    }

    /// Sends a message to the server.
    async fn send_message(&mut self, message: &McuMessage) {
        let mut lock = self.to_server.lock().await;
        let buf = lock.send().await;
        *buf = icd::McuMessage::MotionProfile(message.clone());
        lock.send_done();
    }
}

/// Runs the [`Runner`] forever.
#[embassy_executor::task]
pub async fn run(runner: Runner) {
    runner.run().await;
}

//! This module contains the functionality for running motion profiles sent by the host PC.

#[cfg(feature = "uart_over_adapter")]
use defmt::{error, info};

use crate::{
    LOOP_PERIOD, RunnerRequestReceiver, RunnerResponseSenderMutex,
    gpio::{
        encoder::{ENCODER_STATE, EncoderState, calculate_average_rpm},
        pwm::{SETPOINT_LIST_LENGTH, cubic_conversion},
    },
    pid::{Pid, neg_error},
};
use embassy_executor::task;
use embassy_time::{Instant, Ticker, Timer};
use esp_hal::{mcpwm::operator::PwmPin, peripherals::MCPWM0};
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
    pub const fn new(
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
        'outer: loop {
            self.clear();
            let option = self.setup().await;
            // Don't start until the RPM is 0.
            loop {
                // Clear the buffer.
                // If the motor isn't running, the RPM will be 0 the next time it is checked.
                ENCODER_STATE.with(EncoderState::reset);
                Timer::after_secs(1).await;
                // Check for stop requests.
                if self.should_stop() {
                    continue 'outer;
                }
                if ENCODER_STATE.with(|s| calculate_average_rpm(&s.rpm_ring_buffer)) == 0 {
                    break;
                }
            }
            if let Some(setpoint) = option {
                self.execute_single_rpm(&setpoint).await;
            } else {
                self.execute_motion_profile().await;
            }
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
        let mut ticker = Ticker::every(LOOP_PERIOD);
        // Feedforward
        // We can get the feedforward for the entire run.
        let setpoint_duty_cycle = cubic_conversion(setpoint.rpm);

        // Feedback
        // Initialize the controller
        let mut pi_controller = Pid::new();

        loop {
            // Check for stop requests.
            if self.should_stop() {
                break;
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
            let output = pi_controller.next_control_output(negative_rpm_error);
            let duty_cycle = DutyCycle::from(setpoint_duty_cycle + output);

            // Limit to half of the max power to avoid tripping overcurrent protection
            self.pwm_pin.set_timestamp(duty_cycle.min(HALF_POWER_DUTY));

            // Logging
            let state = State {
                setpoint_rpm: setpoint.rpm,
                current_rpm,
                rpm_error: negative_rpm_error.saturating_neg(),
                duty_cycle,
                time: time_since_start_micros,
            };
            self.send_message(&McuMessage::State(state)).await;

            // Sleep
            ticker.next().await;
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
        let mut ticker = Ticker::every(LOOP_PERIOD);
        let mut setpoint_idx = 0;

        // Feedback
        // Initialize the controller
        let mut pi_controller = Pid::new();

        loop {
            // Check for stop requests.
            if self.should_stop() {
                break;
            }

            let elapsed_since_start_micros = starting_time.elapsed().as_micros();

            // Feedforward
            // Get next pair of setpoints.
            let Some((previous_setpoint, current_setpoint)) =
                self.next_setpoint_pair(&mut setpoint_idx, elapsed_since_start_micros)
            else {
                break;
            };

            // Get setpoint rpm.
            let Some(setpoint_rpm) = Self::next_setpoint_rpm(
                previous_setpoint,
                current_setpoint,
                elapsed_since_start_micros,
            ) else {
                break;
            };
            // Then we need to interpolate to find the required duty cycle.
            let setpoint_duty_cycle = cubic_conversion(setpoint_rpm);

            // Feedback
            let current_rpm =
                ENCODER_STATE.with(|state| calculate_average_rpm(&state.rpm_ring_buffer));
            let rpm_error = neg_error(setpoint_rpm, current_rpm);
            let output = pi_controller.next_control_output(rpm_error);
            let duty_cycle = DutyCycle::from(setpoint_duty_cycle + output);

            // Limit to half of the max power to avoid tripping overcurrent protection
            self.pwm_pin.set_timestamp(duty_cycle.min(HALF_POWER_DUTY));

            // Logging
            let state = State {
                setpoint_rpm,
                current_rpm,
                rpm_error,
                duty_cycle,
                time: elapsed_since_start_micros,
            };
            self.send_message(&McuMessage::State(state)).await;

            // Sleep
            ticker.next().await;
        }
        // Disable PWM
        self.pwm_pin.set_timestamp(STOP_DUTY);
        // Report that the motion profile is finished.
        self.send_message(&McuMessage::Finished).await;
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
            #[allow(
                clippy::arithmetic_side_effects,
                reason = "Incrementing the index by 1 will never overflow because `self.setpoints.get(usize::MAX)` will always return [`None`] before that can happen"
            )]
            let next_setpoint_idx = *setpoint_idx + 1;
            let Some(current_setpoint) = self.setpoints.get(next_setpoint_idx) else {
                #[cfg(feature = "uart_over_adapter")]
                info!("Finished the motion profile.");
                return None;
            };
            let previous_setpoint = self
                .setpoints
                .get(*setpoint_idx)
                .unwrap_or_else(|| unreachable!("next_setpoint_idx > setpoint_idx"));
            // Only act on setpoints that haven't passed.
            if elapsed_since_start_micros <= current_setpoint.time {
                return Some((previous_setpoint, current_setpoint));
            }
            *setpoint_idx = next_setpoint_idx;
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
        /// As long as setpoint RPMs are [`u16`]s,
        /// [`i32`] subtraction between them will not overflow.
        const _: Setpoint = Setpoint { rpm: 0u16, time: 0 };
        #[allow(clippy::arithmetic_side_effects, reason = "See proof above")]
        // The multiplication that occurs later can overflow [`i32`]s, so we need an [`i64`].
        let delta_rpm =
            i64::from(i32::from(current_setpoint.rpm) - i32::from(previous_setpoint.rpm));
        #[allow(
            clippy::arithmetic_side_effects,
            reason = "Sorting and [`next_setpoint_pair`] guarantee that `elapsed_since_start_micros` > `previous_setpoint.time`"
        )]
        let Ok(delta_time) = i64::try_from(elapsed_since_start_micros - previous_setpoint.time)
        else {
            #[cfg(feature = "uart_over_adapter")]
            error!("delta_time doesn't fit in an i64");
            return None;
        };
        let Some(numerator) = delta_rpm.checked_mul(delta_time) else {
            #[cfg(feature = "uart_over_adapter")]
            error!("Overflow in multiplication");
            return None;
        };
        // Sorting and [`next_setpoint_pair`] guarantee that `current_setpoint.time` > `previous_setpoint.time`.
        #[allow(
            clippy::arithmetic_side_effects,
            reason = "Sorting and [`next_setpoint_pair`] guarantee that `current_setpoint.time` > `previous_setpoint.time`"
        )]
        let Ok(denominator) = i64::try_from(current_setpoint.time - previous_setpoint.time) else {
            #[cfg(feature = "uart_over_adapter")]
            error!("Setpoint time difference doesn't fit in an i64");
            return None;
        };
        #[allow(
            clippy::arithmetic_side_effects,
            reason = "Sorting and [`next_setpoint_pair`] guarantee that `denominator` > 0"
        )]
        let interpolation = numerator / denominator;
        let Ok(interpolation) = i16::try_from(interpolation) else {
            #[cfg(feature = "uart_over_adapter")]
            error!("`interpolation` doesn't fit in an i16");
            return None;
        };
        let Some(result) = previous_setpoint.rpm.checked_add_signed(interpolation) else {
            #[cfg(feature = "uart_over_adapter")]
            error!("`previous_setpoint.rpm` + `interpolation` overflowed");
            return None;
        };
        Some(result)
    }

    /// Sends a message to the server.
    async fn send_message(&self, message: &McuMessage) {
        let mut lock = self.to_server.lock().await;
        let buf = lock.send().await;
        *buf = icd::McuMessage::MotionProfile(message.clone());
        lock.send_done();
    }

    /// Returns `true` if the host sent a stop request. Discards all other messages from the host.
    fn should_stop(&mut self) -> bool {
        while let Some(message) = self.from_server.try_receive() {
            let should_stop = matches!(message, HostMessage::Stop);
            self.from_server.receive_done();
            if should_stop {
                return true;
            }
        }
        false
    }
}

/// Runs the [`Runner`] forever.
#[task]
pub async fn run(runner: Runner) {
    runner.run().await;
}

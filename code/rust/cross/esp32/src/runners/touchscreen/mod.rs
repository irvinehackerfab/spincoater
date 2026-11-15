//! This module contains the functionality for running the spincoater with the touchscreen.

pub mod channel;

use core::ops::Add;

use crate::{
    gpio::{
        display::terminal::channel::{TerminalSender, TuiEvent},
        encoder::{ENCODER_STATE, EncoderState, plate_to_motor_revolutions},
        pwm::cubic_conversion,
    },
    pid::{Pid, neg_error},
    runners::LOOP_PERIOD,
};
use channel::{RunAt, RunnerReceiver, RunnerRequest};
use embassy_executor::task;
use embassy_time::{Duration, Instant, Ticker, Timer};
use esp_hal::{mcpwm::operator::PwmPin, peripherals::MCPWM0};
use sc_messages::pwm::{DutyCycle, HALF_POWER_DUTY, STOP_DUTY};

/// The time between motor/time updates on the terminal.
const LOG_PERIOD: Duration = Duration::from_millis(250);

/// The runner that executes single RPM values.
pub struct Runner {
    pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
    from_terminal: RunnerReceiver,
    to_terminal: TerminalSender,
}

impl Runner {
    /// Creates the runner.
    #[must_use]
    pub const fn new(
        pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
        from_terminal: RunnerReceiver,
        to_terminal: TerminalSender,
    ) -> Self {
        Self {
            pwm_pin,
            from_terminal,
            to_terminal,
        }
    }

    /// Runs the main control loop.
    ///
    /// # Panics
    /// Panics if the encoder is not in the mutex.
    pub async fn run(mut self) -> ! {
        'outer: loop {
            if let RunnerRequest::Run(run_at) = self.from_terminal.receive().await {
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
                    if ENCODER_STATE.with(|s| s.rpm) == 0 {
                        break;
                    }
                }
                self.execute(run_at).await;
            }
        }
    }

    /// Executes the run request,
    /// logging info every iteration and checking for a stop command.
    async fn execute(&mut self, run_at: RunAt) {
        let starting_time = Instant::now();
        let mut ticker = Ticker::every(LOOP_PERIOD);
        let mut previous_log = starting_time;
        // Feedforward
        // First we need to convert from plate rpm to motor rpm.
        let setpoint_rpm = plate_to_motor_revolutions(run_at.rpm);
        let setpoint_duty_cycle = cubic_conversion(setpoint_rpm);

        // Feedback
        // Initialize the controller
        let mut pi_controller = Pid::new();

        loop {
            // Check for stop requests.
            if self.should_stop() {
                break;
            }

            // Check if we finished.
            let time_since_start_secs = starting_time
                .elapsed()
                .as_secs()
                .try_into()
                .unwrap_or(u16::MAX);
            if time_since_start_secs >= run_at.time {
                break;
            }

            // Feedback
            let current_rpm = ENCODER_STATE.with(|state| state.rpm);
            let negative_rpm_error = neg_error(setpoint_rpm, current_rpm);
            let output = pi_controller.next_control_output(negative_rpm_error);
            let duty_cycle = DutyCycle::from(setpoint_duty_cycle.add(output));

            // Limit to half of the max power to avoid tripping overcurrent protection
            self.pwm_pin.set_timestamp(duty_cycle.min(HALF_POWER_DUTY));

            // Logging
            if previous_log.elapsed() > LOG_PERIOD {
                let state = RunAt::new(current_rpm, time_since_start_secs);
                self.to_terminal.send(TuiEvent::Runner(state)).await;
                previous_log = Instant::now();
            }

            // Sleep
            ticker.next().await;
        }
        // Disable PWM
        self.pwm_pin.set_timestamp(STOP_DUTY);
        // Report that there is no more state.
        self.to_terminal.send(TuiEvent::RunnerFinished).await;
    }

    /// Returns `true` if the host sent a stop request. Discards all other messages from the host.
    fn should_stop(&mut self) -> bool {
        while let Ok(message) = self.from_terminal.try_receive() {
            if matches!(message, RunnerRequest::Stop) {
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

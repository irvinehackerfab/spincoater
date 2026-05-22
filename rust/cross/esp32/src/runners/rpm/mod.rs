//! This module contains the functionality for running the spincoater at a single rpm for a certain time.

pub mod channel;

use crate::{
    gpio::{
        display::terminal::channel::{TerminalSender, TuiEvent},
        encoder::{
            ENCODER_STATE, EncoderState, calculate_rpm, motor_to_plate_revolutions,
            plate_to_motor_revolutions,
        },
        pwm::linear_conversion,
    },
    pid::{error, next_control_output},
    runners::sleep,
};
use channel::{RunAt, RunnerReceiver, RunnerRequest};
use embassy_time::{Duration, Instant};
use esp_hal::{mcpwm::operator::PwmPin, peripherals::MCPWM0};
use heapless::HistoryBuf;
use sc_messages::pwm::{HALF_POWER_DUTY, STOP_DUTY};
use static_cell::ConstStaticCell;

/// The size of the RPM vector.
///
/// This is currently set to roughly 0.5 seconds worth of RPM readings at [`LOOP_PERIOD`].
const RPM_VEC_SIZE: usize = 64;

/// A list of rpm values for sending an average to the terminal.
pub static RPM_BUFFER: ConstStaticCell<HistoryBuf<usize, RPM_VEC_SIZE>> =
    ConstStaticCell::new(HistoryBuf::new());

/// The time between motor/time updates on the terminal.
const LOG_PERIOD: Duration = Duration::from_millis(500);

/// The runner that executes single RPM values.
pub struct Runner {
    pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
    from_terminal: RunnerReceiver,
    to_terminal: TerminalSender,
    rpm_buffer: &'static mut HistoryBuf<usize, RPM_VEC_SIZE>,
}

impl Runner {
    /// Creates the runner.
    #[must_use]
    pub fn new(
        pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
        from_terminal: RunnerReceiver,
        to_terminal: TerminalSender,
        rpm_buffer: &'static mut HistoryBuf<usize, RPM_VEC_SIZE>,
    ) -> Self {
        Self {
            pwm_pin,
            from_terminal,
            to_terminal,
            rpm_buffer,
        }
    }

    /// Runs the main control loop.
    pub async fn run(mut self) -> ! {
        loop {
            if let RunnerRequest::Run(run_at) = self.from_terminal.receive().await {
                // Since we are starting again, we must reset the encoder state.
                ENCODER_STATE.with(EncoderState::reset);
                self.execute(run_at).await;
            }
        }
    }

    /// Executes the run request,
    /// logging info every iteration and checking for a stop command.
    #[allow(clippy::cast_possible_truncation)]
    async fn execute(&mut self, run_at: RunAt) {
        let starting_time = Instant::now();
        let mut previous_sleep_end = starting_time;
        let mut previous_log = starting_time;
        // Feedforward
        // First we need to convert from plate rpm to motor rpm.
        let setpoint_rpm = plate_to_motor_revolutions(run_at.rpm);
        let setpoint_duty_cycle = linear_conversion(setpoint_rpm);

        loop {
            // Sleep must be called at the start so LOOP_PERIOD time can pass before the current rpm is calculated.
            previous_sleep_end = sleep(previous_sleep_end).await;

            // Check for stop requests.
            if let Ok(RunnerRequest::Stop) = self.from_terminal.try_receive() {
                break;
            }

            // Check if we finished.
            let time_since_start_secs = starting_time.elapsed().as_secs() as u16;
            if time_since_start_secs >= run_at.time {
                break;
            }

            // Feedback
            let current_rpm = ENCODER_STATE.with(|state| calculate_rpm(&state.rpm_ring_buffer));
            let rpm_error = error(setpoint_rpm, current_rpm);
            let output = next_control_output(rpm_error);
            let duty_cycle = (*setpoint_duty_cycle)
                .saturating_add_signed(output)
                .clamp(STOP_DUTY, HALF_POWER_DUTY);

            self.pwm_pin.set_timestamp(duty_cycle);

            // Logging
            self.rpm_buffer
                .write(usize::from(motor_to_plate_revolutions(current_rpm)));
            if previous_log.elapsed() > LOG_PERIOD {
                let average_rpm = calculate_rpm(self.rpm_buffer);
                let state = RunAt::new(average_rpm, time_since_start_secs);
                self.to_terminal.send(TuiEvent::Runner(state)).await;
                previous_log = Instant::now();
            }
        }
        self.pwm_pin.set_timestamp(STOP_DUTY);
        // Report that there is no more state.
        self.to_terminal.send(TuiEvent::RunnerFinished).await;
    }
}

/// Runs the [`Runner`] forever.
#[embassy_executor::task]
pub async fn run(runner: Runner) {
    runner.run().await;
}

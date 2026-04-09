use core::sync::atomic::Ordering;

use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Receiver, Sender},
};
use embassy_time::{Instant, Timer};
use esp_hal::{mcpwm::operator::PwmPin, peripherals::MCPWM0};
use heapless::Vec;
use mc_esp32::{
    APP_PERIOD,
    gpio::{
        display::terminal::channel::{TERMINAL_CHANNEL_SIZE, TuiEvent, send_event_or_report},
        encoder::PLATE_RPM,
        pwm::{THROTTLE_CURVE, THROTTLE_POINTS},
    },
    wifi::channel::{HANDLER_CHANNEL_SIZE, send_info_or_report},
};
use sc_messages::{
    Command, DutyCycle, Info,
    motion_profile::{self, MAX_SETPOINTS, Setpoint},
};

/// The state of the main control loop.
pub struct App {
    setpoints: &'static mut Vec<Setpoint, MAX_SETPOINTS>,
    pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
    from_all: Receiver<'static, NoopRawMutex, Command, HANDLER_CHANNEL_SIZE>,
    to_socket: Sender<'static, NoopRawMutex, Info, HANDLER_CHANNEL_SIZE>,
    to_terminal: Sender<'static, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
}

impl App {
    pub fn new(
        setpoints: &'static mut Vec<Setpoint, MAX_SETPOINTS>,
        pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
        from_all: Receiver<'static, NoopRawMutex, Command, HANDLER_CHANNEL_SIZE>,
        to_socket: Sender<'static, NoopRawMutex, Info, HANDLER_CHANNEL_SIZE>,
        to_terminal: Sender<'static, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
    ) -> Self {
        Self {
            setpoints,
            pwm_pin,
            from_all,
            to_socket,
            to_terminal,
        }
    }

    /// Runs the main control loop.
    pub async fn run(mut self) -> ! {
        loop {
            self.setup().await;
            self.execute_motion_profile().await;
        }
    }

    /// Sets up the motion profile.
    ///
    /// Repeatedly waits for setpoints until a start message is received.
    async fn setup(&mut self) {
        loop {
            match self.from_all.receive().await {
                Command::Add(setpoint) => {
                    let _ = self.setpoints.push(setpoint);
                }
                Command::Start => break,
                Command::Stop => {}
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
        let mut setpoint_idx = 0;
        'outer: loop {
            if let Ok(Command::Stop) = self.from_all.try_receive() {
                break;
            }
            let elapsed = starting_time.elapsed();
            let elapsed_micros = elapsed.as_micros();
            let (previous_setpoint, current_setpoint) = loop {
                match (
                    self.setpoints.get(setpoint_idx),
                    self.setpoints.get(setpoint_idx + 1),
                ) {
                    (Some(previous_setpoint), Some(current_setpoint)) => {
                        // Only act on setpoints that haven't passed.
                        // The times cannot be equal or else we will divide by zero later.
                        if elapsed_micros < previous_setpoint.time {
                            break (previous_setpoint, current_setpoint);
                        }
                        setpoint_idx += 1;
                    }
                    // The motion profile is done.
                    (_, None) | (None, _) => break 'outer,
                };
            };
            // First, we need the setpoint rpm value corresponding to the current time.
            // We need to increase the size of some numbers to prevent overflow.
            // [Wikipedia explanation](https://en.wikipedia.org/wiki/Linear_interpolation#Linear_interpolation_as_an_approximation)
            let previous_setpoint_rpm = previous_setpoint.rpm as u64;
            let current_setpoint_rpm = current_setpoint.rpm as u64;
            let delta_rpm = current_setpoint_rpm - previous_setpoint_rpm;
            let delta_time = elapsed_micros - previous_setpoint.time;
            let numerator = delta_rpm * delta_time;
            let denominator = current_setpoint.time - previous_setpoint.time;
            let setpoint_rpm = u16::try_from(previous_setpoint_rpm + numerator / denominator)
                .expect("RPM should not exceed u16::MAX.");
            // Then we need to linearly interpolate to find the required duty cycle.
            let setpoint_duty_cycle = linear_interpolation(setpoint_rpm);
            self.pwm_pin.set_timestamp(setpoint_duty_cycle);

            // todo!("Add feedback")
            let current_rpm = PLATE_RPM.load(Ordering::Relaxed);

            // Logging
            let duty_cycle = DutyCycle::try_from(setpoint_duty_cycle)
                .expect("Duty cycle should be less than PERIOD.");
            let state = motion_profile::State {
                setpoint_rpm,
                current_rpm,
                duty_cycle,
                time: elapsed_micros,
            };
            send_info_or_report(
                &self.to_socket,
                Info::State(state),
                &self.to_terminal,
                mc_esp32::gpio::display::terminal::channel::ChannelKind::SendInfo,
            )
            .await;
            send_event_or_report(&self.to_terminal, TuiEvent::DutyChanged(duty_cycle)).await;
            // This could be improved by accounting for the execution time of the loop iteration,
            // but this is fine for now.
            Timer::after(APP_PERIOD).await;
        }
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
            return duty_0 + ((duty_1 - duty_0) * (setpoint_rpm - rpm_0)) / (rpm_1 - rpm_0);
        }
    }
    return THROTTLE_CURVE[1][THROTTLE_POINTS - 1];
}

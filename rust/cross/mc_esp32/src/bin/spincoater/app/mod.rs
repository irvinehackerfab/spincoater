use core::sync::atomic::Ordering;

use defmt::info;
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Receiver, Sender},
};
use embassy_time::{Duration, Instant, Timer};
use esp_hal::{mcpwm::operator::PwmPin, peripherals::MCPWM0};
use heapless::Vec;
use mc_esp32::{
    LOOP_PERIOD,
    gpio::{
        display::terminal::channel::{
            ChannelKind, TERMINAL_CHANNEL_SIZE, TuiEvent, send_event_or_report,
        },
        encoder::MOTOR_REVOLUTIONS_DOUBLED,
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
            // Clear the setpoints list, but keep the 0 rpm 0 time element.
            self.setpoints.truncate(1);
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
        let mut previous_iteration = starting_time;
        let mut setpoint_idx = 0;
        'outer: loop {
            // This is prone to accumulating oversleep, but that's not important here.
            Timer::after(LOOP_PERIOD).await;
            if let Ok(Command::Stop) = self.from_all.try_receive() {
                break;
            }
            let elapsed_since_start = starting_time.elapsed();
            let elapsed_since_start_micros = elapsed_since_start.as_micros();
            let (previous_setpoint, current_setpoint) = loop {
                match (
                    self.setpoints.get(setpoint_idx),
                    self.setpoints.get(setpoint_idx + 1),
                ) {
                    (Some(previous_setpoint), Some(current_setpoint)) => {
                        // Only act on setpoints that haven't passed.
                        if elapsed_since_start_micros <= current_setpoint.time {
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
            let delta_time = elapsed_since_start_micros - previous_setpoint.time;
            let numerator = delta_rpm * delta_time;
            let denominator = current_setpoint.time - previous_setpoint.time;
            let setpoint_rpm = u16::try_from(previous_setpoint_rpm + numerator / denominator)
                .expect("RPM should not exceed u16::MAX.");
            // Then we need to linearly interpolate to find the required duty cycle.
            let setpoint_duty_cycle = linear_interpolation(setpoint_rpm);
            self.pwm_pin.set_timestamp(setpoint_duty_cycle);

            // todo!("Add feedback")
            let elapsed_since_last_iteration = previous_iteration.elapsed();
            let current_rpm = self.calculate_rpm(elapsed_since_last_iteration);

            // Logging
            let duty_cycle = DutyCycle::try_from(setpoint_duty_cycle)
                .expect("Duty cycle should be less than PERIOD.");
            let state = motion_profile::State {
                setpoint_rpm,
                current_rpm,
                duty_cycle,
                time: elapsed_since_start_micros,
            };
            send_info_or_report(
                &self.to_socket,
                Info::State(state),
                &self.to_terminal,
                ChannelKind::SendInfo,
            )
            .await;
            send_event_or_report(
                &self.to_terminal,
                TuiEvent::MotionProfileUpdate {
                    duty_cycle,
                    rpm: current_rpm,
                },
            )
            .await;
            previous_iteration += elapsed_since_last_iteration;
        }
        info!("Motion profile done.");
    }

    /// Calculates the current rpm.
    fn calculate_rpm(&self, elapsed_since_last_iteration: Duration) -> u16 {
        // Relaxed ordering because the order of instructions does not matter for the swap.
        let motor_revolutions_doubled = MOTOR_REVOLUTIONS_DOUBLED.swap(0, Ordering::Relaxed);
        let time_ms = u32::try_from(elapsed_since_last_iteration.as_millis())
            .expect("20 milliseconds should fit in a u32.");
        // Avoid dividing by zero
        if time_ms == 0 {
            return 0;
        }
        // (2*motor revolutions) * 1/2 * (20 plate revolutions / 74 motor revolutions) * 1/(`time` ms) * (6000 ms / 1 min)
        // = (2*motor revolutions) * 30,000 / (37 * `time`)
        // Final units: plate revolutions per minute
        let rpm = motor_revolutions_doubled * 30_000 / (37 * (time_ms));
        let rpm = u16::try_from(rpm).expect("The rpm should never exceed 65535.");
        return rpm;
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
            let delta_duty = (duty_1 - duty_0) as u32;
            let delta_rpm = (setpoint_rpm - rpm_0) as u32;
            let numerator = delta_duty * delta_rpm;
            let denominator = (rpm_1 - rpm_0) as u32;
            let result =
                u16::try_from(numerator / denominator).expect("The rpm should never exceed 65535.");
            return duty_0 + result;
        }
    }
    return THROTTLE_CURVE[1][THROTTLE_POINTS - 1];
}

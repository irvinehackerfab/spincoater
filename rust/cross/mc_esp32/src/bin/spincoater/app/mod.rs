use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Receiver, Sender},
};
use embassy_time::{Duration, Instant, Timer};
use esp_hal::{mcpwm::operator::PwmPin, peripherals::MCPWM0};
use heapless::Deque;
use mc_esp32::{
    APP_PERIOD,
    gpio::{
        display::terminal::channel::{
            ChannelKind, TERMINAL_CHANNEL_SIZE, TuiEvent, send_event_or_report,
        },
        pwm::SETPOINTS,
    },
    wifi::channel::{HANDLER_CHANNEL_SIZE, send_info_or_report},
};
use sc_messages::{
    Command, Info, STOP_DUTY,
    motion_profile::{MAX_SETPOINTS, Setpoint},
};

/// The state of the main control loop.
pub struct App {
    setpoints: &'static mut Deque<Setpoint, MAX_SETPOINTS>,
    pwm_pin: PwmPin<'static, MCPWM0<'static>, 0, true>,
    from_all: Receiver<'static, NoopRawMutex, Command, HANDLER_CHANNEL_SIZE>,
    to_socket: Sender<'static, NoopRawMutex, Info, HANDLER_CHANNEL_SIZE>,
    to_terminal: Sender<'static, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
}

impl App {
    pub fn new(
        setpoints: &'static mut Deque<Setpoint, MAX_SETPOINTS>,
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
                    let _ = self.setpoints.push_back(setpoint);
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
        'outer: loop {
            if let Ok(Command::Stop) = self.from_all.try_receive() {
                break;
            }
            let elapsed = starting_time.elapsed();
            let current_setpoint = loop {
                match self.setpoints.pop_front() {
                    Some(setpoint) => {
                        // Only act on setpoints that haven't passed.
                        if elapsed.as_ticks() < setpoint.time {
                            break setpoint;
                        }
                    }
                    None => break 'outer,
                };
            };
            todo!("Add feedforward and feedback");
            todo!("Send duty cycle to terminal and setpoint, state and duty cycle to socket");

            let _ = self.setpoints.push_front(current_setpoint);
            // This could be improved by accounting for the execution time of the loop,
            // but this is fine for now.
            Timer::after(APP_PERIOD).await;
        }
    }
}

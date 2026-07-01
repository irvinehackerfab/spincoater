//! This module contains functionality for the terminal on the display.

pub mod channel;
pub mod ui;

use embedded_graphics::prelude::Point;
use esp_hal::gpio::Output;
use mousefood::{EmbeddedBackend, prelude::Rgb565};
use ratatui::Terminal;
use static_cell::StaticCell;

use crate::{
    gpio::display::{
        DisplayType, HEIGHT, WIDTH,
        terminal::channel::{TerminalReceiver, TuiEvent},
    },
    runners::rpm::channel::{RunAt, RunnerRequest, RunnerSender},
};

/// The static cell for the terminal.
///
/// [`update_terminal`] needs to borrow the terminal because passing it by value would copy 348 bytes.
pub static TERMINAL: StaticCell<Terminal<EmbeddedBackend<DisplayType, Rgb565>>> = StaticCell::new();

/// The default plate RPM.
const RPM: u16 = 5000;

/// The default time in seconds.
const TIME: u16 = 10;

/// The state of the terminal.
pub struct TerminalState {
    /// The vacuum pump pin.
    vacuum_pump_pin: Output<'static>,
    /// A receiver of events.
    from_all: TerminalReceiver,
    /// A sender of requests to the runner.
    to_runner: RunnerSender,
    /// Whether the spincoater is running.
    is_running: bool,
    /// The most recent touch input.
    touch_point: Option<Point>,
    /// The rpm setting in plate RPM.
    target_rpm: u16,
    /// The time setting in seconds.
    target_time: u16,
    /// The current rpm in plate RPM.
    rpm: Option<u16>,
    /// The current time in seconds.
    time: Option<u16>,
}

impl TerminalState {
    /// Creates the terminal.
    #[must_use]
    pub fn new(
        vacuum_pump_pin: Output<'static>,
        from_all: TerminalReceiver,
        to_runner: RunnerSender,
    ) -> Self {
        Self {
            vacuum_pump_pin,
            from_all,
            to_runner,
            is_running: false,
            touch_point: None,
            target_rpm: RPM,
            target_time: TIME,
            rpm: None,
            time: None,
        }
    }

    /// Runs the terminal loop.
    async fn run(
        &mut self,
        terminal: &'static mut Terminal<EmbeddedBackend<'static, DisplayType, Rgb565>>,
    ) -> ! {
        loop {
            let _ = terminal.draw(|frame| self.draw(frame));
            match self.from_all.receive().await {
                TuiEvent::Point(point) => self.handle_touch(point).await,
                TuiEvent::Runner(run_at) => {
                    self.rpm = Some(run_at.rpm);
                    self.time = Some(run_at.time);
                }
                TuiEvent::RunnerFinished => {
                    self.rpm = None;
                    self.time = None;
                    self.is_running = false;
                }
            }
        }
    }

    /// Handles a touch event.
    async fn handle_touch(&mut self, point: Point) {
        const MIDDLE_X: i32 = WIDTH / 2;
        const FIRST_THIRD_Y: i32 = HEIGHT / 3;
        const SECOND_THIRD_Y: i32 = HEIGHT * 2 / 3;

        if self.is_running {
            match (point.x, point.y) {
                (0..MIDDLE_X, SECOND_THIRD_Y..) => {
                    self.to_runner.send(RunnerRequest::Stop).await;
                    self.is_running = false;
                }
                (MIDDLE_X.., SECOND_THIRD_Y..) => {
                    self.vacuum_pump_pin.toggle();
                }
                _ => {}
            }
        } else {
            match (point.x, point.y) {
                (0..MIDDLE_X, 0..FIRST_THIRD_Y) => {
                    self.target_rpm = self.target_rpm.saturating_sub(100);
                }
                (MIDDLE_X.., 0..FIRST_THIRD_Y) => {
                    self.target_rpm = self.target_rpm.saturating_add(100);
                }
                (0..MIDDLE_X, FIRST_THIRD_Y..SECOND_THIRD_Y) => {
                    self.target_time = self.target_time.saturating_sub(1);
                }
                (MIDDLE_X.., FIRST_THIRD_Y..SECOND_THIRD_Y) => {
                    self.target_time = self.target_time.saturating_add(1);
                }
                (0..MIDDLE_X, SECOND_THIRD_Y..) => {
                    self.to_runner
                        .send(RunnerRequest::Run(RunAt::new(
                            self.target_rpm,
                            self.target_time,
                        )))
                        .await;
                    self.is_running = true;
                }
                (MIDDLE_X.., SECOND_THIRD_Y..) => {
                    self.vacuum_pump_pin.toggle();
                }
                _ => {}
            }
        }
        self.touch_point.replace(point);
    }
}

/// This task updates the terminal whenever another task requests it to.
#[embassy_executor::task]
pub async fn update_terminal(
    mut terminal_state: TerminalState,
    terminal: &'static mut Terminal<EmbeddedBackend<'static, DisplayType, Rgb565>>,
) -> ! {
    terminal_state.run(terminal).await
}

//! This module contains functionality for the terminal on the display.
pub mod ui;
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Receiver},
};
use mousefood::{EmbeddedBackend, prelude::Rgb565};
use ratatui::Terminal;
use static_cell::{ConstStaticCell, StaticCell};

use crate::{gpio::display::DisplayType, wifi::WifiState};

/// The static cell for the terminal.
pub static TERMINAL: StaticCell<Terminal<EmbeddedBackend<DisplayType, Rgb565>>> = StaticCell::new();

/// The maximum number of messages allowed at a time in each channel to/from the terminal.
pub const TERMINAL_CHANNEL_SIZE: usize = 8;
/// Used for passing messages from the wifi and socket handler to the terminal.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
/// This does not use a zerocopy channel because [`WifiState`] is currently smaller than a reference.
pub static TERMINAL_CHANNEL: ConstStaticCell<
    Channel<NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
> = ConstStaticCell::new(Channel::new());

/// All possible messages sent to the terminal.
#[derive(Debug)]
pub enum TuiEvent {
    /// The wifi state changed.
    WifiEvent(WifiState),
    /// The PWM output duty cycle changed.
    DutyChanged(u16),
    /// A value for plate revolutions per minute has been calculated.
    RpmValue(u16),
}

/// The state of the terminal.
#[derive(Debug, Default)]
pub struct TerminalState {
    /// The current state of the wifi.
    wifi_state: WifiState,
    /// The current PWM output duty cycle.
    duty: u16,
    /// Plate revolutions per minute.
    rpm: u16,
}

/// This task updates the terminal whenever another task requests it to.
#[embassy_executor::task]
pub async fn update_terminal(
    terminal: &'static mut Terminal<EmbeddedBackend<'static, DisplayType, Rgb565>>,
    from_wifi: Receiver<'static, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
) -> ! {
    let mut state = TerminalState::default();
    loop {
        terminal
            .draw(|frame| state.draw(frame))
            .expect("Failed to draw to terminal");
        match from_wifi.receive().await {
            TuiEvent::WifiEvent(wifi_state) => state.wifi_state = wifi_state,
            TuiEvent::DutyChanged(duty) => state.duty = duty,
            TuiEvent::RpmValue(rpm) => state.rpm = rpm,
        }
    }
}

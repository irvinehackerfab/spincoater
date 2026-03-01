//! This module contains functionality for the terminal on the display.
pub mod channel;
pub mod ui;

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Receiver};
use mousefood::{EmbeddedBackend, prelude::Rgb565};
use ratatui::Terminal;
use static_cell::StaticCell;

use crate::{
    gpio::display::{
        DisplayType,
        terminal::channel::{ChannelStatus, TERMINAL_CHANNEL_SIZE, TuiEvent},
    },
    wifi::WifiState,
};

/// The static cell for the terminal.
pub static TERMINAL: StaticCell<Terminal<EmbeddedBackend<DisplayType, Rgb565>>> = StaticCell::new();

/// The state of the terminal.
#[derive(Debug, Default)]
pub struct TerminalState {
    /// The current state of the wifi.
    wifi_state: WifiState,
    /// The current PWM output duty cycle.
    duty: u16,
    /// Plate revolutions per minute.
    rpm: u16,
    /// Information about the [`embassy_sync::channel::Channel`]s we use.
    channel_status: ChannelStatus,
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
            TuiEvent::ChannelFull(channel_kind) => state.channel_status.set_full(channel_kind),
        }
    }
}

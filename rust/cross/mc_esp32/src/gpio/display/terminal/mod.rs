//! This module contains functionality for the terminal on the display.
use mousefood::{EmbeddedBackend, prelude::Rgb565};
use ratatui::Terminal;
use static_cell::StaticCell;

use crate::gpio::display::DisplayType;

/// The static cell for the terminal.
pub static TERMINAL: StaticCell<Terminal<EmbeddedBackend<DisplayType, Rgb565>>> = StaticCell::new();

/// This task updates the terminal whenever another task requests it to.
#[embassy_executor::task]
pub async fn update_terminal(
    terminal: &'static mut Terminal<EmbeddedBackend<'static, DisplayType, Rgb565>>,
) {
    todo!("terminal.draw()")
}

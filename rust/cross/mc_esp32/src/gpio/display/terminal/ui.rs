//! This module contains the UI description for the terminal.

use ratatui::{Frame, widgets::Paragraph};

use crate::gpio::display::terminal::TerminalState;

impl TerminalState {
    /// Draws the current information to the terminal.
    pub fn draw(&self, frame: &mut Frame) {
        let text = Paragraph::new("Test");
        frame.render_widget(text, frame.area());
    }
}

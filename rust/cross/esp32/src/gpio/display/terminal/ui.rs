//! This module contains the UI description for the terminal.

use esp_alloc::HEAP;
use ratatui::{
    text::{Line, Text, ToSpan},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};

use crate::gpio::display::terminal::TerminalState;

impl TerminalState {
    /// Draws the current information to the terminal.
    pub fn draw(&self, frame: &mut Frame) {
        let block = Block::new().title("esp32");

        let optional_error = {
            match &self.server_error {
                Some(err) => err.to_span(),
                None => "None".to_span(),
            }
        };

        let heap_stats = HEAP.stats();

        let paragraph = Paragraph::new(Text::from_iter([
            // Debug info
            Line::raw("Debug info:"),
            Line::raw("Last Server Error:"),
            Line::from(optional_error),
            {
                Line::from_iter([
                    "Heap usage (bytes): ".to_span(),
                    heap_stats.current_usage.to_span(),
                    " / ".to_span(),
                    heap_stats.size.to_span(),
                ])
            },
        ]))
        .block(block)
        .wrap(Wrap::default());
        frame.render_widget(paragraph, frame.area());
    }
}

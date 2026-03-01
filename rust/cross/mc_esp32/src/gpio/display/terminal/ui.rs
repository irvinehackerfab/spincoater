//! This module contains the UI description for the terminal.

use esp_alloc::HEAP;
use ratatui::{
    Frame,
    text::{Line, Text, ToLine, ToSpan},
    widgets::{Block, Paragraph},
};

use crate::gpio::display::terminal::TerminalState;

impl TerminalState {
    /// Draws the current information to the terminal.
    pub fn draw(&self, frame: &mut Frame) {
        let block = Block::new().title("Irvine Hacker Fab: Spincoater");

        let heap_stats = HEAP.stats();

        let paragraph = Paragraph::new(Text::from_iter([
            self.ap_state.to_line(),
            self.socket_state.to_line(),
            Line::from_iter(["Duty cycle: ".to_span(), self.duty.to_span()]),
            Line::from_iter(["Plate RPM: ".to_span(), self.rpm.to_span()]),
            // Debug info
            Line::raw("\nDebug info:"),
            Line::from_iter([
                "RECV_MSG_CHANNEL was full: ".to_span(),
                self.channel_status.recv_msg_channel_was_full.to_span(),
            ]),
            Line::from_iter([
                "SEND_MSG_CHANNEL was full: ".to_span(),
                self.channel_status.send_msg_channel_was_full.to_span(),
            ]),
            Line::from_iter([
                "TERMINAL_CHANNEL was full: ".to_span(),
                self.channel_status.terminal_channel_was_full.to_span(),
            ]),
            {
                Line::from_iter([
                    "Heap usage (bytes): ".to_span(),
                    heap_stats.current_usage.to_span(),
                    " / ".to_span(),
                    heap_stats.size.to_span(),
                ])
            },
        ]))
        .block(block);
        frame.render_widget(paragraph, frame.area());
    }
}

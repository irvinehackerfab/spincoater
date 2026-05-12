//! This module contains the UI description for the terminal.

use esp_hal::gpio::Level;
use ratatui::{
    Frame,
    layout::{Constraint, HorizontalAlignment, Layout},
    text::{Line, Text, ToSpan},
    widgets::{Block, Paragraph},
};

use crate::gpio::display::terminal::TerminalState;

impl TerminalState {
    /// Draws the current information to the terminal.
    pub fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        let layout = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]);
        let [main_area, footer_area] = area.layout(&layout);

        if let Some(touch_point) = self.touch_point {
            let footer = Text::from(Line::from_iter([
                "(".to_span(),
                touch_point.x.to_span(),
                ", ".to_span(),
                touch_point.y.to_span(),
                ")".to_span(),
            ]))
            .centered();
            frame.render_widget(footer, footer_area);
        } else {
            let footer = Text::from("Irvine Hacker Fab").centered();
            frame.render_widget(footer, footer_area);
        }

        let main_layout = Layout::vertical([Constraint::Ratio(1, 3); 3]);
        let [rpm_area, time_area, bottom_area] = main_area.layout(&main_layout);
        let rpm_block = Block::bordered()
            .title(Line::from_iter(["RPM: ".to_span(), self.rpm.to_span()]))
            .title_alignment(HorizontalAlignment::Center);

        let time_block = Block::bordered()
            .title(Line::from_iter(["Time: ".to_span(), self.time.to_span()]))
            .title_alignment(HorizontalAlignment::Center);

        if !self.is_running {
            let rpm_block_inner = rpm_block.inner(rpm_area);
            frame.render_widget(rpm_block, rpm_area);

            let rpm_layout = Layout::horizontal([Constraint::Ratio(1, 2); 2]);
            let [rpm_decrease_area, rpm_increase_area] = rpm_block_inner.layout(&rpm_layout);

            let rpm_decrease = Block::bordered();
            let rpm_decrease_text = Paragraph::new("-100").centered().block(rpm_decrease);
            frame.render_widget(rpm_decrease_text, rpm_decrease_area);

            let rpm_increase = Block::bordered();
            let rpm_increase_text = Paragraph::new("+100").centered().block(rpm_increase);
            frame.render_widget(rpm_increase_text, rpm_increase_area);

            let time_block_inner = time_block.inner(time_area);
            frame.render_widget(time_block, time_area);

            let time_layout = Layout::horizontal([Constraint::Ratio(1, 2); 2]);
            let [time_decrease_area, time_increase_area] = time_block_inner.layout(&time_layout);

            let time_decrease = Block::bordered();
            let time_decrease_text = Paragraph::new("-1").centered().block(time_decrease);
            frame.render_widget(time_decrease_text, time_decrease_area);

            let time_increase = Block::bordered();
            let time_increase_text = Paragraph::new("+1").centered().block(time_increase);
            frame.render_widget(time_increase_text, time_increase_area);
        }

        let bottom_layout = Layout::horizontal([Constraint::Ratio(1, 2); 2]);
        let [start_stop_area, vacuum_pump_area] = bottom_area.layout(&bottom_layout);

        let start_stop_block = Block::bordered();
        let start_stop_text = Paragraph::new(if self.is_running { "Stop" } else { "Start" })
            .centered()
            .block(start_stop_block);
        frame.render_widget(start_stop_text, start_stop_area);

        let vacuum_pump_block = Block::bordered().title("Vacuum Pump");
        match self.vacuum_pump_pin.output_level() {
            Level::High => {
                let vacuum_pump_text = Paragraph::new("Disable")
                    .centered()
                    .block(vacuum_pump_block);
                frame.render_widget(vacuum_pump_text, vacuum_pump_area);
            }
            Level::Low => {
                let vacuum_pump_text = Paragraph::new("Enable").centered().block(vacuum_pump_block);
                frame.render_widget(vacuum_pump_text, vacuum_pump_area);
            }
        }
    }
}

//! This module describes the UI layout of the terminal.

use ratatui::{
    Frame,
    layout::{Constraint, HorizontalAlignment, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, List, ListItem, ListState, Paragraph},
};
use ringbuffer::RingBuffer;
use sc_messages::PERIOD;

use crate::app::App;

impl App {
    /// Renders the user interface widgets.
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let layout = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]);
        let [main_area, footer_area] = area.layout(&layout);

        let footer = Text::from("Irvine Hacker Fab").centered();
        frame.render_widget(footer, footer_area);

        let main_layout = Layout::horizontal([Constraint::Ratio(1, 2); 2]);
        let [left_half, right_half] = main_area.layout(&main_layout);
        let right_half_layout = Layout::vertical([Constraint::Ratio(1, 2); 2]);
        let [upper_right, lower_right] = right_half.layout(&right_half_layout);

        self.render_commands(left_half, frame);
        self.render_state(upper_right, frame);
        self.render_logs(lower_right, frame);
    }

    fn render_commands(&mut self, area: Rect, frame: &mut Frame) {
        Self::render_command_list(&mut self.commands_state, area, frame);
    }

    fn render_command_list(list_state: &mut ListState, area: Rect, frame: &mut Frame) {
        let instructions = Line::from_iter([
            " Up: ".into(),
            "<Up>".blue().bold(),
            " Down: ".into(),
            "<Down>".blue().bold(),
            " Select: ".into(),
            "<Enter>".blue().bold(),
            " Exit: ".into(),
            "<Esc>,<Ctrl+C>,<q> ".blue().bold(),
        ]);

        let cmd_block = Block::bordered()
            .title(" Commands ")
            .title_alignment(HorizontalAlignment::Center)
            .border_type(BorderType::Rounded)
            .title_bottom(instructions);

        let items = ["Load motion profile CSV file", "Start", "Stop"];
        let list = List::new(items)
            .block(cmd_block)
            .highlight_symbol("-> ")
            .highlight_style(Style::new().blue());

        frame.render_stateful_widget(list, area, list_state);
    }

    fn render_state(&mut self, area: Rect, frame: &mut Frame) {
        let block = Block::bordered()
            .title(" MCU State ")
            .title_alignment(HorizontalAlignment::Center)
            .border_type(BorderType::Rounded);

        let paragraph = Paragraph::new(Text::from_iter([
            Line::raw(format!("Current RPM: {}", self.state_rpm)),
            Line::raw(format!("Current setpoint RPM: {}", self.setpoint_rpm)),
            Line::raw(format!("Duty Cycle (0..{}): {}", PERIOD, self.duty_cycle)),
            Line::raw(format!(
                "Duty Cycle (0.0..1.0): {}",
                f32::from(*self.duty_cycle) / f32::from(PERIOD)
            )),
        ]))
        .block(block);

        frame.render_widget(paragraph, area);
    }

    fn render_logs(&mut self, area: Rect, frame: &mut Frame) {
        let info_block = Block::bordered()
            .title("Previous Commands")
            .title_alignment(HorizontalAlignment::Center)
            .border_type(BorderType::Rounded);

        // Todo: Consider replacing with scrolled paragraph to gain wrapping support
        let items = self
            .previous_commands
            .iter()
            .map(|command| ListItem::new(Text::from(format!("{command:?}"))));

        let list = List::new(items).block(info_block);
        let mut state = ListState::default();
        state.select_last();

        frame.render_stateful_widget(list, area, &mut state);
    }
}

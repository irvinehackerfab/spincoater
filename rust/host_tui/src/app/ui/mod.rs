//! This module describes the UI layout of the terminal.
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, HorizontalAlignment, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};
use ringbuffer::RingBuffer;
use sc_messages::PERIOD;

use crate::app::App;

impl Widget for &mut App {
    /// Renders the user interface widgets.
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]);
        let [main_area, footer_area] = area.layout(&layout);

        let footer = Text::from("Irvine Hacker Fab").centered();
        footer.render(footer_area, buf);

        let main_layout = Layout::horizontal([Constraint::Ratio(1, 2); 2]);
        let [left_half, right_half] = main_area.layout(&main_layout);
        let right_half_layout = Layout::vertical([Constraint::Ratio(1, 2); 2]);
        let [upper_right, lower_right] = right_half.layout(&right_half_layout);

        self.render_commands(left_half, buf);
        self.render_state(upper_right, buf);
        self.render_logs(lower_right, buf);
    }
}

impl App {
    fn render_commands(&mut self, area: Rect, buf: &mut Buffer) {
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

        let items = ["Set Duty Cycle to 5%", "Set Duty Cycle to 10%"];
        let list = List::new(items)
            .block(cmd_block)
            .highlight_symbol("-> ")
            .highlight_style(Style::new().blue());

        StatefulWidget::render(list, area, buf, &mut self.commands_state);
    }

    fn render_state(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title(" MCU State ")
            .title_alignment(HorizontalAlignment::Center)
            .border_type(BorderType::Rounded);

        Paragraph::new(Text::from_iter([
            Line::raw(format!("Duty Cycle (0..{}): {}", PERIOD, self.duty_cycle.0)),
            Line::raw(format!(
                "Duty Cycle (0.0..1.0): {}",
                f32::from(self.duty_cycle.0) / f32::from(PERIOD)
            )),
        ]))
        .block(block)
        .render(area, buf);
    }

    fn render_logs(&mut self, area: Rect, buf: &mut Buffer) {
        let info_block = Block::bordered()
            .title(self.log_file_path.as_str())
            .title_alignment(HorizontalAlignment::Center)
            .border_type(BorderType::Rounded);

        // Todo: Consider replacing with scrolled paragraph to gain wrapping support
        let items = self
            .messages
            .iter()
            .map(|msg| ListItem::new(Text::from(msg.to_string())));

        let list = List::new(items).block(info_block);
        let mut state = ListState::default();
        state.select_last();

        StatefulWidget::render(list, area, buf, &mut state);
    }
}

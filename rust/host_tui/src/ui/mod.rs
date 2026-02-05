use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, List, ListItem, ListState, StatefulWidget, Widget},
};

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

        self.render_commands(left_half, buf);
        self.render_info(right_half, buf);
    }
}

impl App {
    fn render_commands(&mut self, area: Rect, buf: &mut Buffer) {
        let instructions = Line::from(vec![
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
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded)
            .title_bottom(instructions);

        let items = ["Set Duty Cycle to 5%", "Set Duty Cycle to 10%"];
        let list = List::new(items)
            .block(cmd_block)
            .highlight_symbol("-> ")
            .highlight_style(Style::new().blue());

        StatefulWidget::render(list, area, buf, &mut self.commands_state);
    }

    fn render_info(&mut self, area: Rect, buf: &mut Buffer) {
        let info_block = Block::bordered()
            .title(" Messages to/from MCU ")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let items = self.messages.iter().map(|msg| {
            ListItem::new(Text::from(format!(
                "{} -- {}: {}",
                msg.timestamp.format("%m-%d-%Y %H:%M:%S:%f"),
                if msg.from_mcu { "From MCU" } else { "To MCU" },
                msg.message
            )))
        });

        let list = List::new(items).block(info_block);
        let mut state = ListState::default();
        state.select_last();

        StatefulWidget::render(list, area, buf, &mut state);
    }
}

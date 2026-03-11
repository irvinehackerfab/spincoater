//! This module describes the UI layout of the terminal.

use ratatui::{
    Frame,
    layout::{Constraint, Flex, HorizontalAlignment, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, List, ListItem, ListState, Paragraph},
};
use ringbuffer::RingBuffer;
use sc_messages::{MAX_POWER_DUTY, PERIOD, STOP_DUTY};
use tui_input::Input;

use crate::app::App;

use super::CommandsState;

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
        match &mut self.commands_state {
            CommandsState::List(list_state) => Self::render_command_list(list_state, area, frame),
            CommandsState::Input { input, last_error } => {
                Self::render_command_prompt(input, last_error.as_ref(), area, frame);
            }
        }
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

        let items = ["Set Duty Cycle", "Stop Motor"];
        let list = List::new(items)
            .block(cmd_block)
            .highlight_symbol("-> ")
            .highlight_style(Style::new().blue());

        frame.render_stateful_widget(list, area, list_state);
    }

    fn render_command_prompt(
        input: &Input,
        last_error: Option<&String>,
        area: Rect,
        frame: &mut Frame,
    ) {
        let layout =
            Layout::vertical([Constraint::Length(5), Constraint::Length(4)]).flex(Flex::Start);
        let [top_half, bottom_half] = area.layout(&layout);

        let cmd_block = Block::bordered()
            .title(" Commands ")
            .title_alignment(HorizontalAlignment::Center)
            .border_type(BorderType::Rounded);
        let paragraph = Paragraph::new(Text::from_iter([
            format!("Please input a duty cycle in the range of 0..{PERIOD}.",),
            format!("Duty cycle to stop: {STOP_DUTY}"),
            format!("Duty cycle for max speed: {MAX_POWER_DUTY}"),
        ]))
        .block(cmd_block);

        let instructions = Line::from_iter([
            " Set: ".into(),
            "<Enter>".blue().bold(),
            " Cancel: ".into(),
            "<Esc> ".blue().bold(),
        ]);
        let input_block = Block::bordered()
            .title(" Input ")
            .title_alignment(HorizontalAlignment::Center)
            .title_bottom(instructions);
        let input_style = Style::new().blue();
        let text = match last_error {
            Some(err) => Text::from_iter([
                Line::styled(input.value(), input_style),
                Line::styled(err, Style::new().red()),
            ]),
            None => Text::styled(input.value(), input_style),
        };
        // keep 2 for borders and 1 for cursor
        let width = bottom_half.width.max(3) - 3;
        let scroll = input.visual_scroll(usize::from(width));
        #[allow(clippy::cast_possible_truncation)]
        let input_paragraph = Paragraph::new(text)
            .block(input_block)
            .scroll((0, scroll as u16));

        // Ratatui hides the cursor unless it's explicitly set.
        // Position the cursor past the end of the input text and one line down from the border to the input line.
        let x = input.visual_cursor().max(scroll) - scroll + 1;
        #[allow(clippy::cast_possible_truncation)]
        frame.set_cursor_position((bottom_half.x + x as u16, bottom_half.y + 1));

        frame.render_widget(paragraph, top_half);
        frame.render_widget(input_paragraph, bottom_half);
    }

    fn render_state(&mut self, area: Rect, frame: &mut Frame) {
        let block = Block::bordered()
            .title(" MCU State ")
            .title_alignment(HorizontalAlignment::Center)
            .border_type(BorderType::Rounded);

        let paragraph = Paragraph::new(Text::from_iter([
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

        frame.render_stateful_widget(list, area, &mut state);
    }
}

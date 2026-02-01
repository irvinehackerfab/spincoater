use crate::event::{EventHandler, TuiEvent};
use chrono::{DateTime, Local};
use crossterm::event::Event;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use sc_messages::Message;
use tokio::net::TcpStream;

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Event handler.
    pub events: EventHandler,
    /// The state of the commands section.
    pub commands_state: ListState,
    /// All messages sent to the PC since the app started.
    pub messages: Vec<MessageInfo>,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(stream: TcpStream) -> Self {
        Self {
            running: true,
            events: EventHandler::new(stream),
            commands_state: ListState::default().with_selected(Some(0)),
            messages: (0..20).map(|i| Message::DutyCycle(i).into()).collect(),
        }
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            match self.events.next().await? {
                TuiEvent::Crossterm(event) => match event {
                    Event::Key(key_event)
                        if key_event.kind == crossterm::event::KeyEventKind::Press =>
                    {
                        self.handle_key_events(key_event)?
                    }
                    // We're only concerned with key presses right now.
                    _ => {}
                },
                TuiEvent::Wireless(message_info) => self.messages.push(message_info),
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.running = false,
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.running = false
            }
            KeyCode::Up => self.commands_state.scroll_up_by(1),
            KeyCode::Down => self.commands_state.scroll_down_by(1),
            KeyCode::Enter => todo!("Add command sending"),
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }

    // /// Handles the key events and updates the state of [`App`].
    // fn handle_wireless_message(&mut self, message_info: MessageInfo) -> color_eyre::Result<()> {
    //     match message_info.message {
    //         // If it's a duty cycle report, add it to the log
    //         // TODO: Also write it to a permanent log file
    //         Message::DutyCycle(_) => self
    //             .messages
    //             .push(MessageInfo::new(message_info, Instant::now())),
    //     }
    //     Ok(())
    // }
}

#[derive(Debug, Clone)]
pub struct MessageInfo {
    pub message: Message,
    pub timestamp: DateTime<Local>,
}

impl From<Message> for MessageInfo {
    fn from(message: Message) -> Self {
        Self {
            message,
            timestamp: Local::now(),
        }
    }
}

// #[derive(Debug, Default)]
// struct WirelessInfoState {
//     vertical_scroll_state: ScrollbarState,
//     horizontal_scroll_state: ScrollbarState,
// }

// #[derive(Debug, Default)]
// enum SelectedBlock {
//     #[default]
//     Commands,
//     WirelessInfo,
// }

// impl SelectedBlock {
//     /// Changes self to the next variant, wrapping around at the end.
//     fn next(&mut self) {
//         match self {
//             SelectedBlock::Commands => *self = SelectedBlock::WirelessInfo,
//             SelectedBlock::WirelessInfo => *self = SelectedBlock::Commands,
//         }
//     }
// }

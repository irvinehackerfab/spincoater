use crate::event::{EventHandler, TuiEvent};
use chrono::{DateTime, Local};
use color_eyre::{Result, eyre::OptionExt};
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
    /// All messages sent from/to the MCU since the app started.
    pub messages: Vec<MessageInfo>,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(stream: TcpStream) -> Self {
        Self {
            running: true,
            events: EventHandler::new(stream),
            commands_state: ListState::default().with_selected(Some(0)),
            messages: Vec::new(),
        }
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            match self.events.next().await?? {
                TuiEvent::Crossterm(event) => match event {
                    Event::Key(key_event)
                        if key_event.kind == crossterm::event::KeyEventKind::Press =>
                    {
                        self.handle_key_events(key_event).await?
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
    async fn handle_key_events(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.running = false,
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.running = false
            }
            KeyCode::Up => self.commands_state.scroll_up_by(1),
            KeyCode::Down => self.commands_state.scroll_down_by(1),
            KeyCode::Enter => match self
                .commands_state
                .selected()
                .ok_or_eyre("One command is always selected")?
            {
                // Send 5% duty cycle command to the MCU and add it to the log.
                0 => self
                    .messages
                    .push(self.events.send(Message::DutyCycle(5)).await?),
                // Send 10% duty cycle command to the MCU and add it to the log.
                1 => self
                    .messages
                    .push(self.events.send(Message::DutyCycle(10)).await?),
                _ => {}
            },
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MessageInfo {
    pub message: Message,
    pub from_mcu: bool,
    pub timestamp: DateTime<Local>,
}

impl MessageInfo {
    /// Adds information to a message that was received from the MCU.
    pub fn from_mcu(message: Message) -> Self {
        Self {
            message,
            from_mcu: true,
            timestamp: Local::now(),
        }
    }

    /// Adds information to a message right after it was sent to the MCU.
    pub fn to_mcu(message: Message) -> Self {
        Self {
            message,
            from_mcu: false,
            timestamp: Local::now(),
        }
    }
}

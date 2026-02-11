use std::fmt::{Display, Formatter};
use std::fs::{DirBuilder, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs::File, io::BufWriter};

use crate::event::{EventHandler, TuiEvent};
use chrono::{DateTime, Local};
use color_eyre::{Result, eyre::OptionExt};
use crossterm::event::Event;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use sc_messages::Message;
use tokio::net::TcpStream;

const MESSAGE_CAPACITY: usize = 100;

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Event handler.
    pub events: EventHandler,
    /// The state of the commands section.
    pub commands_state: ListState,
    /// The last [`MESSAGE_CAPACITY`] messages sent from/to the MCU since the app started.
    pub messages: AllocRingBuffer<MessageInfo>,
    pub log_file: BufWriter<File>,
    pub log_file_path: String,
}

impl App {
    /// Constructs a new instance of [`App`].
    ///
    /// # Errors
    /// Returns an error if opening the stream fails or opening the log file fails.
    pub fn new(stream: TcpStream) -> Result<Self> {
        let (log_file_path, log_file) =
            Self::open_log_file("sc_logs", Local::now().date_naive().to_string())?;

        let log_file_path = log_file_path.to_string_lossy().into_owned();

        Ok(Self {
            running: true,
            events: EventHandler::new(stream),
            commands_state: ListState::default().with_selected(Some(0)),
            messages: AllocRingBuffer::new(MESSAGE_CAPACITY),
            log_file,
            log_file_path,
        })
    }

    fn open_log_file(directory: &str, date: String) -> Result<(PathBuf, BufWriter<File>)> {
        let mut dir = env::current_dir()?;
        dir.push(directory);
        DirBuilder::new().recursive(true).create(dir.clone())?;
        dir.push(date);
        dir.add_extension("txt");
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(dir.clone())?;
        let buf_file = BufWriter::new(file);
        Ok((dir, buf_file))
    }

    /// Run the application's main loop.
    ///
    /// # Errors
    /// Returns an error if drawing to the terminal, receiving events or handling keystrokes fails.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            match self.events.next().await?? {
                TuiEvent::Crossterm(event) => match event {
                    Event::Key(key_event)
                        if key_event.kind == crossterm::event::KeyEventKind::Press =>
                    {
                        self.handle_key_events(key_event).await?;
                    }
                    // We're only concerned with key presses right now.
                    _ => {}
                },
                TuiEvent::Wireless(message_info) => {
                    writeln!(self.log_file, "{message_info}")?;
                    let _ = self.messages.enqueue(message_info);
                }
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    async fn handle_key_events(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.running = false,
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.running = false;
            }
            KeyCode::Up => self.commands_state.scroll_up_by(1),
            KeyCode::Down => self.commands_state.scroll_down_by(1),
            KeyCode::Enter => match self
                .commands_state
                .selected()
                .ok_or_eyre("One command is always selected")?
            {
                // Send 5% duty cycle command to the MCU.
                0 => self.events.send(Message::DutyCycle(5)).await?,
                // Send 10% duty cycle command to the MCU.
                1 => self.events.send(Message::DutyCycle(10)).await?,
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

impl Display for MessageInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} -- {}",
            self.timestamp.format("%m-%d-%Y %H:%M:%S"),
            self.message
        )
    }
}

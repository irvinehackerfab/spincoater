//! This module contains the app representing the TUI.
pub mod event;
pub mod ui;

use std::fmt::{Display, Formatter};
use std::fs::{DirBuilder, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs::File, io::BufWriter};

use crate::app::event::{EventHandler, TuiEvent};
use cfg_if::cfg_if;
use chrono::{DateTime, Local};
use color_eyre::{Result, eyre::OptionExt};
use crossterm::event::Event;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use sc_messages::{MAX_POWER_DUTY, Message, STOP_DUTY};
use tokio::net::TcpStream;

/// The maximum number of messages kept in the TUI at a time
/// (all messages are written to the log file.)
///
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
    ///
    /// When max capacity is reached, the oldest messages are overridden.
    pub messages: AllocRingBuffer<MessageInfo>,
    /// The log file.
    pub log_file: BufWriter<File>,
    /// The log file path.
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

    /// Opens the log file.
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
                0 => {
                    let message = Message::DutyCycle(STOP_DUTY);
                    self.events.send(message).await?;
                    let message_info = MessageInfo::new(message, false);
                    writeln!(self.log_file, "{message_info}")?;
                    let _ = self.messages.enqueue(message_info);
                }
                // Send 10% duty cycle command to the MCU.
                1 => {
                    let message = Message::DutyCycle(MAX_POWER_DUTY);
                    self.events.send(message).await?;
                    let message_info = MessageInfo::new(message, false);
                    writeln!(self.log_file, "{message_info}")?;
                    let _ = self.messages.enqueue(message_info);
                }
                _ => {}
            },
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }
}

/// A message, with the time it was received.
#[derive(Debug, Clone)]
pub struct MessageInfo {
    message: Message,
    timestamp: DateTime<Local>,
    from_mcu: bool,
}

impl MessageInfo {
    #[must_use]
    pub fn new(message: Message, from_mcu: bool) -> Self {
        Self {
            message,
            timestamp: Local::now(),
            from_mcu,
        }
    }
}

cfg_if! {
    if #[cfg(feature = "dev-socket")] {
        impl Display for MessageInfo {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{} (fake socket) -- {}: {}",
                    self.timestamp.format("%m-%d-%Y %H:%M:%S"),
                    if self.from_mcu { "From MCU" } else { "To MCU" },
                    self.message
                )
            }
        }
    } else {
        impl Display for MessageInfo {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{} -- {}: {}",
                    self.timestamp.format("%m-%d-%Y %H:%M:%S"),
                    if self.from_mcu { "From MCU" } else { "To MCU" },
                    self.message
                )
            }
        }
    }
}

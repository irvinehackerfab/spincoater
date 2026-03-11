//! This module contains the app representing the TUI.
pub mod event;
pub mod message;
pub mod ui;

use std::fs::{DirBuilder, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs::File, io::BufWriter};

use crate::app::event::{EventHandler, TuiEvent};
use crate::app::message::MessageInfo;
use chrono::Local;
use color_eyre::{Result, eyre::OptionExt};
use crossterm::event::Event;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use sc_messages::{DutyCycle, Message, STOP_DUTY};
use tokio::net::TcpStream;
use tui_input::Input;

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
    pub commands_state: CommandsState,
    /// The ramp up time of the motor, as reported by the MCU.
    pub duty_cycle: DutyCycle,
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
            duty_cycle: DutyCycle::try_from(0)?,
            commands_state: CommandsState::List(ListState::default().with_selected(Some(0))),
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
            terminal.draw(|frame| self.render(frame))?;
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
                    match message_info.message {
                        Message::DutyCycle(duty_cycle) => self.duty_cycle = duty_cycle,
                    }
                    writeln!(self.log_file, "{message_info}")?;
                    let _ = self.messages.enqueue(message_info);
                }
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    async fn handle_key_events(&mut self, key_event: KeyEvent) -> Result<()> {
        match &mut self.commands_state {
            CommandsState::List(list_state) => match key_event.code {
                KeyCode::Esc | KeyCode::Char('q') => self.running = false,
                KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                    self.running = false;
                }
                KeyCode::Up => list_state.scroll_up_by(1),
                KeyCode::Down => list_state.scroll_down_by(1),
                KeyCode::Enter => match list_state
                    .selected()
                    .ok_or_eyre("One command is always selected")?
                {
                    // Create a prompt for setting the duty cycle.
                    0 => {
                        self.commands_state = CommandsState::Input {
                            input: Input::default(),
                            last_error: None,
                        }
                    }
                    // Stop the motor immediately.
                    1 => {
                        self.send_message(Message::DutyCycle(STOP_DUTY)).await?;
                    }
                    _ => {}
                },
                // Other handlers you could add here.
                _ => {}
            },
            CommandsState::Input { input, last_error } => match key_event.code {
                KeyCode::Enter => match input.value().parse::<u16>() {
                    Ok(duty_cycle) => match DutyCycle::try_from(duty_cycle) {
                        Ok(duty_cycle) => {
                            self.send_message(Message::DutyCycle(duty_cycle)).await?;
                            self.commands_state =
                                CommandsState::List(ListState::default().with_selected(Some(0)));
                        }
                        Err(err) => *last_error = Some(err.to_string()),
                    },
                    Err(err) => *last_error = Some(err.to_string()),
                },
                KeyCode::Esc => {
                    self.commands_state =
                        CommandsState::List(ListState::default().with_selected(Some(0)));
                }
                _ => {
                    let _ = tui_input::backend::crossterm::EventHandler::handle_event(
                        input,
                        &Event::Key(key_event),
                    );
                }
            },
        }
        Ok(())
    }

    /// A reusable method for sending a message and logging it.
    async fn send_message(&mut self, message: Message) -> Result<()> {
        self.events.send(message).await?;
        let message_info = MessageInfo::new(message, false);
        writeln!(self.log_file, "{message_info}")?;
        let _ = self.messages.enqueue(message_info);
        Ok(())
    }
}

/// The current state of the commands section.
#[derive(Debug)]
pub enum CommandsState {
    /// The options are currently listed.
    List(ListState),
    /// The user is being prompted to input a duty cycle.
    Input {
        input: Input,
        last_error: Option<String>,
    },
}

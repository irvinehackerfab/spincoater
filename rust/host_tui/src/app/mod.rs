//! This module contains the app representing the TUI.
pub mod event;
pub mod ui;

use std::fs::{DirBuilder, OpenOptions};
use std::io::{self};
use std::path::PathBuf;
use std::{env, fs::File};

use crate::app::event::{EventHandler, TuiEvent};
use chrono::Local;
use color_eyre::{Result, eyre::OptionExt};
use crossterm::event::Event;
use csv::{Writer, WriterBuilder};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use sc_messages::motion_profile::Setpoint;
use sc_messages::{Command, DutyCycle};
use tokio::net::TcpStream;

/// The maximum number of commands kept in the TUI at a time.
const COMMAND_CAPACITY: usize = 100;

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Event handler.
    pub events: EventHandler,
    /// The state of the commands section.
    pub commands_state: ListState,
    /// The current plate RPM, as reported by the MCU.
    pub state_rpm: u16,
    /// The current setpoint plate RPM, as reported by the MCU.
    pub setpoint_rpm: u16,
    /// The current duty cycle, as reported by the MCU.
    pub duty_cycle: DutyCycle,
    /// The last [`COMMAND_CAPACITY`] commands sent to the MCU since the app started.
    ///
    /// When max capacity is reached, the oldest messages are overridden.
    pub previous_commands: AllocRingBuffer<Command>,
    /// The log file.
    pub log_file: Writer<File>,
}

impl App {
    /// Constructs a new instance of [`App`].
    ///
    /// # Errors
    /// Returns an error if opening the stream fails or opening the log file fails.
    pub fn new(stream: TcpStream) -> Result<Self> {
        let date = Local::now().date_naive().to_string();
        let log_file = Self::open_log_file(&date)?;

        Ok(Self {
            running: true,
            events: EventHandler::new(stream),
            state_rpm: 0,
            setpoint_rpm: 0,
            duty_cycle: DutyCycle::try_from(0)?,
            commands_state: ListState::default().with_selected(Some(0)),
            previous_commands: AllocRingBuffer::new(COMMAND_CAPACITY),
            log_file,
        })
    }

    /// Opens the log file.
    fn open_log_file(date: &str) -> Result<Writer<File>> {
        const LOG_DIR: &str = "sc_logs";

        let mut dir = env::current_dir()?;
        dir.push(LOG_DIR);
        DirBuilder::new().recursive(true).create(dir.clone())?;
        dir.push(format!("{date}.txt"));
        // If the file already exists, we need to make a new one.
        let mut open_options = OpenOptions::new();
        open_options.read(true).append(true).create_new(true);
        let file = match open_options.open(dir.clone()) {
            Ok(file) => file,
            Err(err) => match err.kind() {
                io::ErrorKind::AlreadyExists => {
                    let mut i = 1;
                    loop {
                        dir.set_file_name(format!("{date}_({i}).txt"));
                        match open_options.open(dir.clone()) {
                            Ok(file) => break file,
                            Err(err) => match err.kind() {
                                io::ErrorKind::AlreadyExists => i += 1,
                                _ => return Err(err.into()),
                            },
                        }
                    }
                }
                _ => return Err(err.into()),
            },
        };
        let writer = WriterBuilder::new().from_writer(file);
        Ok(writer)
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
                TuiEvent::Wireless(info) => {
                    match info {
                        sc_messages::Info::State(state) => self.state_rpm = state.rpm,
                        sc_messages::Info::Setpoint(setpoint) => self.setpoint_rpm = setpoint.rpm,

                        sc_messages::Info::DutyCycle(duty_cycle) => self.duty_cycle = duty_cycle,
                    }
                    self.log_file.serialize(info)?;
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
                // Create a prompt for setting the duty cycle.
                0 => {
                    let path = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .set_directory(env::current_dir()?)
                        .set_title("Please choose a motion profile CSV file.")
                        .pick_file();
                    if let Some(path) = path {
                        self.send_motion_profile(path).await?;
                    }
                }
                // Start the motion profile.
                1 => self.send_command(Command::Start).await?,
                // Stop the motion profile.
                2 => self.send_command(Command::Stop).await?,
                _ => {}
            },
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }

    /// A reusable method for sending a command and logging it.
    async fn send_command(&mut self, command: Command) -> Result<()> {
        self.events.send(command).await?;
        let _ = self.previous_commands.enqueue(command);
        Ok(())
    }

    /// Loads a motion profile from a CSV [`PathBuf`] and sends it.
    async fn send_motion_profile(&mut self, path: PathBuf) -> Result<()> {
        let file = csv::Reader::from_path(path)?;
        for result in file.into_deserialize() {
            let setpoint: Setpoint = result?;
            let command = Command::Add(setpoint);
            self.events.send(command).await?;
            let _ = self.previous_commands.enqueue(command);
        }
        Ok(())
    }
}

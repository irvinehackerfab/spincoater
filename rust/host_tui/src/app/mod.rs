//! This module contains the app representing the TUI.
pub mod event;
pub mod ui;

use std::fs::{DirBuilder, OpenOptions};
use std::io::{self};
use std::path::PathBuf;
use std::{env, fs::File};

use crate::app::event::{EventHandler, TuiEvent, UsbEvent};
use chrono::Local;
use color_eyre::{Result, eyre::OptionExt};
use crossterm::event::Event;
use csv::{Writer, WriterBuilder};
use postcard_rpc::host_client::HostClient;
use postcard_rpc::standard_icd::WireError;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use sc_messages::commands::Command;
use sc_messages::icd::CommandEndpoint;
use sc_messages::motion_profile::Setpoint;
use sc_messages::pwm::DutyCycle;

/// The maximum number of MCU logs kept in the TUI at a time.
pub const MCU_LOG_CAPACITY: usize = 128;

/// Application.
#[derive(Debug)]
pub struct App {
    /// This boolean provides an easy way for methods to end the program.
    running: bool,
    /// The client allows for sending requests to the MCU.
    client: HostClient<WireError>,
    /// Event handler.
    events: EventHandler,
    /// The state of the commands section.
    commands_state: ListState,
    /// The current plate RPM, as reported by the MCU.
    current_rpm: u16,
    /// The current setpoint plate RPM, as reported by the MCU.
    setpoint_rpm: u16,
    /// The current duty cycle, as reported by the MCU.
    duty_cycle: DutyCycle,
    /// The last [`MCU_LOG_CAPACITY`] commands received from the MCU since the app started.
    ///
    /// When max capacity is reached, the oldest messages are overridden.
    mcu_logs: AllocRingBuffer<String>,
    /// The motor data file.
    motor_data_file: Writer<File>,
}

impl App {
    /// Constructs a new instance of [`App`].
    ///
    /// # Errors
    /// Returns an error if opening the log file fails.
    pub async fn new(client: HostClient<WireError>) -> Result<Self> {
        let events = EventHandler::new(client.clone()).await?;
        let date = Local::now().date_naive().to_string();
        let motor_data_file = Self::open_log_file(&date)?;

        Ok(Self {
            running: true,
            client,
            events,
            current_rpm: 0,
            setpoint_rpm: 0,
            duty_cycle: DutyCycle::try_from(0)?,
            commands_state: ListState::default().with_selected(Some(0)),
            mcu_logs: AllocRingBuffer::new(MCU_LOG_CAPACITY),
            motor_data_file,
        })
    }

    /// Opens the motor data log file.
    fn open_log_file(date: &str) -> Result<Writer<File>> {
        const LOG_DIR: &str = "motor_data";

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
                        self.handle_key_event(key_event).await?;
                    }
                    // We're only concerned with key presses right now.
                    _ => {}
                },
                TuiEvent::Usb(usb_event) => self.handle_usb_event(usb_event)?,
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    async fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
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
                // Clear all setpoints.
                1 => self.send_command(&Command::ClearSetpoints).await?,
                // Start the motion profile.
                2 => self.send_command(&Command::Start).await?,
                // Stop the motion profile.
                3 => self.send_command(&Command::Stop).await?,
                _ => {}
            },
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }

    fn handle_usb_event(&mut self, usb_event: UsbEvent) -> Result<()> {
        match usb_event {
            UsbEvent::Log(msg) => {
                let _ = self.mcu_logs.enqueue(msg);
            }
            UsbEvent::State(state) => {
                self.current_rpm = state.current_rpm;
                self.setpoint_rpm = state.setpoint_rpm;
                self.duty_cycle = state.duty_cycle;
                self.motor_data_file.serialize(state)?;
            }
        }
        Ok(())
    }

    /// Loads a motion profile from a CSV [`PathBuf`] and sends it.
    async fn send_motion_profile(&mut self, path: PathBuf) -> Result<()> {
        let file = csv::Reader::from_path(path)?;
        for result in file.into_deserialize() {
            let setpoint: Setpoint = result?;
            let command = Command::Add(setpoint);
            self.send_command(&command).await?;
        }
        Ok(())
    }

    /// A reusable method for sending commands to the MCU and logging any bad responses.
    async fn send_command(&mut self, command: &Command) -> Result<()> {
        match self.client.send_resp::<CommandEndpoint>(command).await? {
            Ok(()) => {}
            Err(err) => {
                let _ = self.mcu_logs.enqueue(format!("Host TUI warning: {err:?}"));
            }
        }
        Ok(())
    }
}

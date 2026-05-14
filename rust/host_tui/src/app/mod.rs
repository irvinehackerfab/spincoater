//! This module contains the app representing the TUI.
pub mod event;
pub mod state;
pub mod ui;

use std::fs::{DirBuilder, OpenOptions};
use std::io::{self};
use std::path::PathBuf;
use std::{env, fs::File};

use crate::app::event::{EventHandler, MCUEvent, TuiEvent};
use crate::app::state::MotionProfileState;
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
use sc_messages::motion_profile::{self, Setpoint};
use sc_messages::vacuum_pump;

/// The maximum number of MCU logs kept in the TUI at a time.
pub const MCU_LOG_CAPACITY: usize = 128;

/// The directory for log files.
pub const LOG_DIR: &str = "logs";

/// The subdirectory for motor data files.
pub const MOTOR_DATA_SUB_DIR: &str = "motor_data";

/// The subdirectory for touchscreen data files.
pub const TOUCHSCREEN_DATA_SUB_DIR: &str = "touchscreen_data";

/// All the state for the host terminal.
#[derive(Debug)]
pub struct App {
    /// This boolean provides an easy way for methods to end the program.
    running: bool,
    /// Event handler.
    events: EventHandler,
    /// The state of the commands section.
    commands_state: ListState,
    /// The current state, as reported by the MCU.
    mcu_state: Option<MotionProfileState>,
    /// The last [`MCU_LOG_CAPACITY`] commands received from the MCU since the app started.
    ///
    /// When max capacity is reached, the oldest messages are overridden.
    mcu_logs: AllocRingBuffer<String>,
    /// The motor data file.
    /// This is only [`Some`] when a motion profile is running.
    motor_data_file: Option<Writer<File>>,
    /// The touchscreen data file.
    touchscreen_data_file: Writer<File>,
}

impl App {
    /// Constructs a new instance of [`App`].
    ///
    /// # Errors
    /// Returns an error if opening the log file fails.
    pub async fn new(client: HostClient<WireError>) -> Result<Self> {
        let events = EventHandler::new(client).await?;
        Ok(Self {
            running: true,
            events,
            mcu_state: None,
            commands_state: ListState::default().with_selected(Some(0)),
            mcu_logs: AllocRingBuffer::new(MCU_LOG_CAPACITY),
            motor_data_file: None,
            touchscreen_data_file: Self::open_log_file(TOUCHSCREEN_DATA_SUB_DIR)?,
        })
    }

    /// Opens a log file.
    fn open_log_file(sub_dir: &str) -> Result<Writer<File>> {
        let mut dir = env::current_dir()?;
        dir.push(LOG_DIR);
        dir.push(sub_dir);

        DirBuilder::new().recursive(true).create(dir.clone())?;
        let date = Local::now().date_naive().to_string();
        dir.push(format!("{date}.csv"));
        // If the file already exists, we need to make a new one.
        let mut open_options = OpenOptions::new();
        open_options.read(true).append(true).create_new(true);
        let file = match open_options.open(dir.clone()) {
            Ok(file) => file,
            Err(err) => match err.kind() {
                io::ErrorKind::AlreadyExists => {
                    let mut i = 1;
                    loop {
                        dir.set_file_name(format!("{date}_({i}).csv"));
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

    /// Runs the application.
    ///
    /// Attempts to disconnect cleanly upon exit.
    ///
    /// # Errors
    /// Returns an error if drawing to the terminal, receiving events or handling keystrokes fails.
    pub async fn run(mut self, terminal: DefaultTerminal) -> Result<()> {
        let result = self.app_loop(terminal).await;
        self.events.send_disconnect_notification().await;
        result
    }

    /// Runs the application's main loop.
    async fn app_loop(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            match self.events.next().await?? {
                TuiEvent::Crossterm(event) => match event {
                    Event::Key(key_event)
                        if key_event.kind == crossterm::event::KeyEventKind::Press =>
                    {
                        self.handle_key_event(key_event)?;
                    }
                    // We're only concerned with key presses right now.
                    _ => {}
                },
                TuiEvent::MCU(usb_event) => self.handle_mcu_event(usb_event)?,
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.running = false;
            }
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
                        self.send_motion_profile(path)?;
                    }
                }
                // Clear all setpoints.
                1 => self
                    .events
                    .send_motion_profile_request(motion_profile::Request::ClearSetpoints),
                // Start the motion profile.
                2 => {
                    self.motor_data_file
                        .replace(Self::open_log_file(MOTOR_DATA_SUB_DIR)?);
                    self.events
                        .send_motion_profile_request(motion_profile::Request::Start);
                }
                // Stop the motion profile.
                3 => self
                    .events
                    .send_motion_profile_request(motion_profile::Request::Stop),
                // Enable the vacuum pump.
                4 => self
                    .events
                    .send_vacuum_pump_request(vacuum_pump::Request::Enable),
                // Disable the vacuum pump.
                5 => self
                    .events
                    .send_vacuum_pump_request(vacuum_pump::Request::Disable),
                _ => {}
            },
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }

    fn handle_mcu_event(&mut self, mcu_event: MCUEvent) -> Result<()> {
        match mcu_event {
            MCUEvent::Log(msg) => {
                let _ = self.mcu_logs.enqueue(format!("[Log]: {msg}"));
            }
            MCUEvent::State(state) => {
                self.mcu_state.clone_from(&state);
                match state {
                    Some(state) => self
                        .motor_data_file
                        .as_mut()
                        .ok_or_eyre("The motor data file should be open.")?
                        .serialize(state)?,
                    None => {
                        // Close the writer.
                        let _ = self.motor_data_file.take();
                    }
                }
            }
            MCUEvent::MotionProfileRequestResponse(response) => {
                let _ = self.mcu_logs.enqueue(format!("{response}"));
            }
            MCUEvent::VacuumPumpRequestResponse => {
                let _ = self.mcu_logs.enqueue("[Vacuum Pump]: Ok".to_string());
            }
            MCUEvent::Touch(touch_point) => {
                let _ = self.mcu_logs.enqueue(format!("[Touch]: {touch_point:?}"));
                self.touchscreen_data_file.serialize(touch_point)?;
            }
        }
        Ok(())
    }

    /// Loads a motion profile from a CSV [`PathBuf`] and sends it.
    ///
    /// Note that [`postcard_rpc`] makes no guarantee about the order in which setpoints are sent,
    /// but the MCU sorts them before execution.
    fn send_motion_profile(&mut self, path: PathBuf) -> Result<()> {
        let file = csv::Reader::from_path(path)?;
        for result in file.into_deserialize() {
            let setpoint: Setpoint = result?;
            let command = motion_profile::Request::Add(setpoint);
            self.events.send_motion_profile_request(command);
        }
        Ok(())
    }
}

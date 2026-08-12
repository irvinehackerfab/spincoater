//! This module contains the app representing the TUI.
pub mod event;
pub mod state;
pub mod ui;

use std::fs::{DirBuilder, OpenOptions};
use std::io::{self};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::{env, fs::File};

use crate::app::event::{
    TuiEvent, spawn_crossterm_thread, spawn_heartbeat_thread, spawn_rx_thread, spawn_tx_thread,
};
use crate::app::state::MotionProfileState;
use chrono::Local;
use color_eyre::eyre::{Context, bail};
use color_eyre::{Result, eyre::OptionExt};
use crossterm::event::Event;
use csv::{StringRecord, Writer, WriterBuilder};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use sc_messages::icd::{HostMessage, MAX_HOST_MESSAGE_SIZE, MAX_MCU_MESSAGE_SIZE, McuMessage};
use sc_messages::motion_profile::{self, Setpoint};
use sc_messages::vacuum_pump::{self};
use serial2::SerialPort;
use static_cell::{ConstStaticCell, StaticCell};

/// The maximum number of MCU logs kept in the TUI at a time.
pub const MCU_LOG_CAPACITY: usize = 128;

/// The directory for log files.
pub const LOG_DIR: &str = "logs";

/// The subdirectory for motor data files.
pub const MOTOR_DATA_SUB_DIR: &str = "motor_data";

/// The subdirectory for touchscreen data files.
pub const TOUCHSCREEN_DATA_SUB_DIR: &str = "touchscreen_data";

/// The static cell for the serial port.
pub static SERIAL_PORT: StaticCell<SerialPort> = StaticCell::new();

/// The buffer used for sending [`HostMessage`]s.
///
/// Use this when calling [`App::new`].
pub static SEND_BUFFER: ConstStaticCell<[u8; MAX_HOST_MESSAGE_SIZE]> = ConstStaticCell::new([0; _]);

/// All the state for the host terminal.
#[derive(Debug)]
pub struct App {
    /// This boolean provides an easy way for methods to end the program.
    running: bool,
    /// The channel for sending messages to the MCU.
    ///
    /// If this is [`None`], the MCU is not running.
    to_mcu: Sender<HostMessage>,
    /// The receiver of [`TuiEvent`]s from the other threads.
    from_all: Receiver<Result<TuiEvent>>,
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
}

impl App {
    /// Constructs a new instance of [`App`].
    ///
    /// # Errors
    /// Returns an error if opening the log file fails.
    pub fn new(
        serial: &'static SerialPort,
        send_buffer: &'static mut [u8; MAX_HOST_MESSAGE_SIZE],
        read_buffer: &'static mut [u8; MAX_MCU_MESSAGE_SIZE],
    ) -> Result<Self> {
        // Setup communication
        let (to_app, from_all) = mpsc::channel::<Result<TuiEvent>>();
        let to_app_2 = to_app.clone();
        let to_app_3 = to_app.clone();
        let (to_mcu, from_app) = mpsc::channel::<HostMessage>();
        let to_mcu_2 = to_mcu.clone();
        // Spawn thread for getting crossterm events
        spawn_crossterm_thread(to_app).wrap_err("Failed to spawn crossterm thread")?;
        spawn_rx_thread(serial, read_buffer, to_app_2).wrap_err("Failed to spawn RX thread")?;
        spawn_tx_thread(serial, send_buffer, to_app_3, from_app)
            .wrap_err("Failed to spawn TX thread")?;
        spawn_heartbeat_thread(to_mcu_2).wrap_err("Failed to spawn heartbeat thread")?;
        Ok(Self {
            running: true,
            from_all,
            to_mcu,
            mcu_state: None,
            commands_state: ListState::default().with_selected(Some(0)),
            mcu_logs: AllocRingBuffer::new(MCU_LOG_CAPACITY),
            motor_data_file: None,
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
    pub fn run(mut self, terminal: DefaultTerminal) -> Result<()> {
        let result = self.app_loop(terminal);
        self.to_mcu
            .send(HostMessage::Disconnecting)
            .wrap_err_with(|| {
                format!("Failed to disconnect cleanly after ending with result: {result:#?}")
            })?;
        result
    }

    /// Runs the application's main loop.
    fn app_loop(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            match self
                .from_all
                .recv()
                .wrap_err("Both event senders were dropped")??
            {
                TuiEvent::Crossterm(event) => match event {
                    Event::Key(key_event)
                        if key_event.kind == crossterm::event::KeyEventKind::Press =>
                    {
                        self.handle_key_event(key_event)?;
                    }
                    // We're only concerned with key presses right now.
                    _ => {}
                },
                TuiEvent::Mcu(message) => self.handle_mcu_message(message)?,
                TuiEvent::Heartbeat => {
                    self.to_mcu
                        .send(HostMessage::Heartbeat)
                        .wrap_err("Failed to send heartbeat")?;
                    self.mcu_logs.enqueue("Sent heartbeat".to_string());
                }
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
                0 if self.mcu_state.is_none() => {
                    let path = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .set_directory(env::current_dir().wrap_err("Failed to get current dir")?)
                        .set_title("Please choose a motion profile CSV file.")
                        .pick_file();
                    if let Some(path) = path {
                        self.send_motion_profile(path)
                            .wrap_err("Failed to send motion profile")?;
                    }
                }
                // Clear all setpoints.
                1 if self.mcu_state.is_none() => self
                    .to_mcu
                    .send(HostMessage::MotionProfile(
                        motion_profile::HostMessage::ClearSetpoints,
                    ))
                    .wrap_err("Failed to clear setpoints")?,
                // Start the motion profile.
                2 if self.mcu_state.is_none() => {
                    self.motor_data_file = Some(
                        Self::open_log_file(MOTOR_DATA_SUB_DIR)
                            .wrap_err("Failed to open log file")?,
                    );
                    self.to_mcu
                        .send(HostMessage::MotionProfile(
                            motion_profile::HostMessage::Start,
                        ))
                        .wrap_err("Failed to start motion profile")?;
                }
                // Get a single setpoint to run the spincoater at.
                3 if self.mcu_state.is_none() => {
                    let path = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .set_directory(env::current_dir().wrap_err("Failed to get current dir")?)
                        .set_title("Please choose a CSV file with a single setpoint.")
                        .pick_file();
                    if let Some(path) = path {
                        self.motor_data_file = Some(
                            Self::open_log_file(MOTOR_DATA_SUB_DIR)
                                .wrap_err("Failed to open log file")?,
                        );
                        self.send_single_setpoint(path)
                            .wrap_err("Failed to send single setpoint")?;
                    }
                }
                // Stop the motion profile.
                4 => self
                    .to_mcu
                    .send(HostMessage::MotionProfile(
                        motion_profile::HostMessage::Stop,
                    ))
                    .wrap_err("Failed to stop motion profile")?,
                // Enable the vacuum pump.
                5 if self.mcu_state.is_none() => self
                    .to_mcu
                    .send(HostMessage::VacuumPump(vacuum_pump::HostMessage::Enable))
                    .wrap_err("Failed to enable vacuum pump")?,
                // Disable the vacuum pump.
                6 if self.mcu_state.is_none() => self
                    .to_mcu
                    .send(HostMessage::VacuumPump(vacuum_pump::HostMessage::Disable))
                    .wrap_err("Failed to disable vacuum pump")?,
                _ => {}
            },
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }

    fn handle_mcu_message(&mut self, message: McuMessage) -> Result<()> {
        match message {
            McuMessage::MotionProfile(mcu_message) => match mcu_message {
                motion_profile::McuMessage::State(state) => {
                    let full_state: MotionProfileState = state.clone().into();
                    self.mcu_state = Some(full_state.clone());
                    self.motor_data_file
                        .as_mut()
                        .ok_or_eyre("The motor data file should be open.")?
                        .serialize(full_state)
                        .wrap_err("Failed to serialize to CSV file")?;
                }
                motion_profile::McuMessage::Finished => {
                    self.mcu_state = None;
                    // Close the writer.
                    self.motor_data_file = None;
                }
            },
            McuMessage::Heartbeat => {}
            McuMessage::Error(error) => bail!("MCU failed: {:#?}", error),
        }
        Ok(())
    }

    /// Loads a motion profile from a CSV [`PathBuf`] and sends it.
    ///
    /// Note that the MCU performs stable sort on the setpoints before execution.
    fn send_motion_profile(&mut self, path: PathBuf) -> Result<()> {
        let file = csv::Reader::from_path(path).wrap_err("Failed to open CSV file")?;
        for result in file.into_deserialize() {
            let setpoint: Setpoint = result.wrap_err("Failed to deserialize from CSV file")?;
            let message = HostMessage::MotionProfile(motion_profile::HostMessage::Add(setpoint));
            self.to_mcu
                .send(message)
                .wrap_err("Failed to send setpoint")?;
        }
        Ok(())
    }

    /// Loads a [`Setpoint`] from a CSV [`PathBuf`] and sends it.
    fn send_single_setpoint(&mut self, path: PathBuf) -> Result<()> {
        let mut file = csv::Reader::from_path(path).wrap_err("Failed to open CSV file")?;
        let mut record = StringRecord::new();
        file.read_record(&mut record)
            .wrap_err("Failed to read CSV")?;
        let setpoint: Setpoint = record.deserialize(None).wrap_err("Failed to deserialize")?;
        let message = HostMessage::MotionProfile(motion_profile::HostMessage::Run(setpoint));
        self.to_mcu
            .send(message)
            .wrap_err("Failed to send setpoint")
    }
}

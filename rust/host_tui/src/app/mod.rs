//! This module contains the app representing the TUI.
pub mod event;
pub mod state;
pub mod ui;

use std::fs::{DirBuilder, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::{env, fs::File};

use crate::app::event::{TuiEvent, spawn_crossterm_thread, spawn_mcu_thread};
use crate::app::state::MotionProfileState;
use chrono::Local;
use color_eyre::eyre::Context;
use color_eyre::{Result, eyre::OptionExt};
use crossterm::event::Event;
use csv::{Writer, WriterBuilder};
use postcard::to_slice_cobs;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use ringbuffer::AllocRingBuffer;
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
    /// The serial connection to the MCU.
    to_mcu: BufWriter<&'static SerialPort>,
    send_buffer: &'static mut [u8; MAX_HOST_MESSAGE_SIZE],
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
        // Spawn thread for getting crossterm events
        spawn_crossterm_thread(to_app).wrap_err("Failed to spawn crossterm thread")?;
        spawn_mcu_thread(serial, read_buffer, to_app_2).wrap_err("Failed to spawn MCU thread")?;
        Ok(Self {
            running: true,
            from_all,
            send_buffer,
            to_mcu: BufWriter::new(serial),
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
        self.send_message(&HostMessage::Disconnecting, true)
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
                    .send_message(
                        &HostMessage::MotionProfile(motion_profile::HostMessage::ClearSetpoints),
                        true,
                    )
                    .wrap_err("Failed to clear setpoints")?,
                // Start the motion profile.
                2 => {
                    self.motor_data_file = Some(
                        Self::open_log_file(MOTOR_DATA_SUB_DIR)
                            .wrap_err("Failed to open log file")?,
                    );
                    self.send_message(
                        &HostMessage::MotionProfile(motion_profile::HostMessage::Start),
                        true,
                    )
                    .wrap_err("Failed to start motion profile")?;
                }
                // Stop the motion profile.
                3 => self
                    .send_message(
                        &HostMessage::MotionProfile(motion_profile::HostMessage::Stop),
                        true,
                    )
                    .wrap_err("Failed to stop motion profile")?,
                // Enable the vacuum pump.
                4 => self
                    .send_message(
                        &HostMessage::VacuumPump(vacuum_pump::HostMessage::Enable),
                        true,
                    )
                    .wrap_err("Failed to enable vacuum pump")?,
                // Disable the vacuum pump.
                5 => self
                    .send_message(
                        &HostMessage::VacuumPump(vacuum_pump::HostMessage::Disable),
                        true,
                    )
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
                    self.mcu_state = Some(state.clone().into());
                    self.motor_data_file
                        .as_mut()
                        .ok_or_eyre("The motor data file should be open.")?
                        .serialize(state)
                        .wrap_err("Failed to serialize to CSV file")?;
                }
                motion_profile::McuMessage::Finished => {
                    self.mcu_state = None;
                    // Close the writer.
                    self.motor_data_file = None;
                }
            },
        }
        Ok(())
    }

    /// Loads a motion profile from a CSV [`PathBuf`] and sends it.
    ///
    /// Note that [`postcard_rpc`] makes no guarantee about the order in which setpoints are sent,
    /// but the MCU sorts them before execution.
    fn send_motion_profile(&mut self, path: PathBuf) -> Result<()> {
        let file = csv::Reader::from_path(path).wrap_err("Failed to read CSV file")?;
        for result in file.into_deserialize() {
            let setpoint: Setpoint = result.wrap_err("Failed to deserialize from CSV file")?;
            let command = HostMessage::MotionProfile(motion_profile::HostMessage::Add(setpoint));
            self.send_message(&command, false)
                .wrap_err("Failed to send motion profile setpoint")?;
        }
        // Flush at the end
        self.to_mcu.flush().wrap_err("Failed to flush")?;
        Ok(())
    }

    /// Sends a single message to the MCU.
    ///
    /// If you plan on sending multiple messages, flush [`App::to_mcu`] at the end.
    fn send_message(&mut self, message: &HostMessage, flush: bool) -> Result<()> {
        let used =
            to_slice_cobs(message, self.send_buffer).wrap_err("Failed to serialize HostMessage")?;
        self.to_mcu
            .write_all(used)
            .wrap_err("Failed to write to serial port")?;
        if flush {
            self.to_mcu.flush().wrap_err("Failed to flush")?;
        }
        Ok(())
    }
}

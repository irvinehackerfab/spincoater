//! This module decribes events that cause updates to the TUI.
use color_eyre::{
    Result,
    eyre::{OptionExt, eyre},
};
use futures::StreamExt;
use postcard_rpc::{
    host_client::{HostClient, Subscription},
    standard_icd::{LoggingTopic, WireError},
};
use ratatui::crossterm::event::Event as CrosstermEvent;
use sc_messages::{icd::MotionProfileStateTopic, motion_profile::State};
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::app::MCU_LOG_CAPACITY;

/// All possible TUI events.
#[derive(Clone, Debug)]
pub enum TuiEvent {
    /// Crossterm events such as keyboard inputs.
    ///
    /// These events are emitted by the terminal.
    Crossterm(CrosstermEvent),
    /// Events from the MCU connection.
    Usb(UsbEvent),
}

/// All possible USB events.
#[derive(Clone, Debug)]
pub enum UsbEvent {
    /// The MCU logged a message.
    Log(String),
    /// The MCU sent the motion profile state.
    State(State),
}

/// Terminal event handler.
#[derive(Debug)]
pub struct EventHandler {
    /// Event receiver channel.
    ///
    /// The tasks themselves hold the senders.
    from_tasks: mpsc::UnboundedReceiver<Result<TuiEvent>>,
}

impl EventHandler {
    /// Constructs a new instance of [`EventHandler`] and spawns tasks to handle events.
    ///
    /// # Errors
    /// Returns an error if subscribing to the necessary topics fails.
    pub async fn new(client: HostClient<WireError>) -> Result<Self> {
        let (to_handler, from_tasks) = mpsc::unbounded_channel();
        // Subscribe to the MCU's logs.
        let log_stream = client
            .subscribe_exclusive::<LoggingTopic>(MCU_LOG_CAPACITY)
            .await?;
        // Subscribe to the MCU's motion profile state.
        let state_stream = client
            .subscribe_exclusive::<MotionProfileStateTopic>(MCU_LOG_CAPACITY)
            .await?;

        // Spawn event handler tasks.
        tokio::spawn(await_crossterm_events(to_handler.clone()));
        tokio::spawn(await_log_messages(log_stream, to_handler.clone()));
        tokio::spawn(await_state_messages(state_stream, to_handler));

        Ok(Self { from_tasks })
    }

    /// Receives an event from the sender.
    ///
    /// This function blocks until an event is received.
    ///
    /// # Errors
    ///
    /// This function returns an error if the sender channel is disconnected. This can happen if an
    /// error occurs in the event thread. In practice, this should not happen unless there is a
    /// problem with the underlying terminal.
    pub async fn next(&mut self) -> Result<Result<TuiEvent>> {
        self.from_tasks
            .recv()
            .await
            .ok_or_eyre("Failed to receive event")
    }
}

/// Sends crossterm events to the terminal whenever they occur.
async fn await_crossterm_events(to_handler: UnboundedSender<Result<TuiEvent>>) {
    let mut reader = crossterm::event::EventStream::new();
    loop {
        if let Some(result) = reader.next().await {
            match result {
                Ok(event) => {
                    // If the channel is closed, this task is done.
                    if to_handler.send(Ok(TuiEvent::Crossterm(event))).is_err() {
                        return;
                    }
                }
                // I'm not sure what these errors are, so for now we will print them and end the program.
                Err(error) => {
                    let _ = to_handler.send(Err(error.into()));
                    return;
                }
            }
        } else {
            // If the stream is closed, this task is done.
            let _ = to_handler.send(Err(eyre!("Crossterm event stream closed")));
            return;
        }
    }
}

/// Sends the MCU's logs to the terminal whenever they occur.
async fn await_log_messages(
    mut subscription: Subscription<String>,
    to_handler: UnboundedSender<Result<TuiEvent>>,
) {
    // As soon as the stream closes, the terminal must close as well.
    while let Some(msg) = subscription.recv().await {
        if to_handler
            .send(Ok(TuiEvent::Usb(UsbEvent::Log(msg))))
            .is_err()
        {
            break;
        }
    }
    let _ = to_handler.send(Err(eyre!("postcard_rpc closed the MCU log stream")));
}

/// Sends the MCU's motion profile state to the terminal.
async fn await_state_messages(
    mut subscription: Subscription<State>,
    to_handler: UnboundedSender<Result<TuiEvent>>,
) {
    // As soon as the stream closes, the terminal must close as well.
    while let Some(state) = subscription.recv().await {
        if to_handler
            .send(Ok(TuiEvent::Usb(UsbEvent::State(state))))
            .is_err()
        {
            break;
        }
    }
    let _ = to_handler.send(Err(eyre!("postcard_rpc closed the MCU log stream")));
}

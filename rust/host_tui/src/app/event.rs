//! This module decribes events that cause updates to the TUI.
use std::{convert::Into, fmt::Display, time::Duration};

use chrono::{Local, NaiveTime};
use color_eyre::{
    Result,
    eyre::{OptionExt, eyre},
};
use futures::StreamExt;
use postcard_rpc::{
    header::VarSeq,
    host_client::{HostClient, Subscription},
    standard_icd::{LoggingTopic, WireError},
};
use ratatui::crossterm::event::Event as CrosstermEvent;
use sc_messages::{
    icd::{
        HostDisconnecting, MotionProfileStateTopic, MotionRequestEndpoint, TouchPointTopic,
        VacuumPumpRequestEndpoint,
    },
    motion_profile::{self, RequestRefused},
    touchscreen::TouchPoint,
    vacuum_pump,
};
use serde::de::DeserializeOwned;
use tokio::{
    sync::mpsc::{self, UnboundedSender},
    time::timeout,
};

use crate::app::{MCU_LOG_CAPACITY, state::MotionProfileState};

/// [`postcard_rpc`] requires us to choose a message sequence number and does not explain why.
const INITIAL_VAR_SEQ: VarSeq = VarSeq::Seq1(0);

/// All possible TUI events.
#[derive(Clone, Debug)]
pub enum TuiEvent {
    /// Crossterm events such as keyboard inputs.
    ///
    /// These events are emitted by the terminal.
    Crossterm(CrosstermEvent),
    /// Events from the MCU connection.
    MCU(MCUEvent),
}

impl From<MCUEvent> for TuiEvent {
    fn from(value: MCUEvent) -> Self {
        Self::MCU(value)
    }
}

/// All possible USB events.
#[derive(Debug, Clone)]
pub enum MCUEvent {
    /// The MCU responded to a motion profile request.
    MotionProfileRequestResponse(Response),
    /// The MCU responded to a vacuum pump request.
    VacuumPumpRequestResponse,
    /// The MCU logged a message.
    Log(String),
    /// The MCU sent the motion profile state.
    State(Option<MotionProfileState>),
    /// The MCU sent a touch input.
    Touch(TouchPoint),
}

impl From<String> for MCUEvent {
    fn from(value: String) -> Self {
        Self::Log(value)
    }
}

impl From<Option<motion_profile::State>> for MCUEvent {
    fn from(value: Option<motion_profile::State>) -> Self {
        Self::State(value.map(Into::into))
    }
}

impl From<TouchPoint> for MCUEvent {
    fn from(value: TouchPoint) -> Self {
        Self::Touch(value)
    }
}

/// A motion profile response + the time it was received.
#[derive(Debug, Clone)]
pub struct Response {
    response: core::result::Result<(), RequestRefused>,
    time: NaiveTime,
}

impl Response {
    fn new(response: core::result::Result<(), RequestRefused>, time: NaiveTime) -> Self {
        Self { response, time }
    }

    /// Returns the response.
    ///
    /// # Errors
    /// Returns an error if the request was refused.
    pub fn response(&self) -> core::result::Result<(), RequestRefused> {
        self.response
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Motion Profile] [{}]: {:?}", self.time, self.response)
    }
}

/// Terminal event handler.
#[derive(Debug)]
pub struct EventHandler {
    /// Event receiver channel.
    ///
    /// The tasks themselves hold the senders.
    from_tasks: mpsc::UnboundedReceiver<Result<TuiEvent>>,
    /// The client allows for sending requests to the MCU.
    client: HostClient<WireError>,
    /// A sender for cloning and using in future tasks.
    to_handler: mpsc::UnboundedSender<Result<TuiEvent>>,
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
        // Subscribe to the MCU's touch points.
        let touch_stream = client
            .subscribe_exclusive::<TouchPointTopic>(MCU_LOG_CAPACITY)
            .await?;

        // Spawn event handler tasks.
        tokio::spawn(await_crossterm_events(to_handler.clone()));
        tokio::spawn(await_messages(log_stream, to_handler.clone()));
        tokio::spawn(await_messages(state_stream, to_handler.clone()));
        tokio::spawn(await_messages(touch_stream, to_handler.clone()));

        Ok(Self {
            from_tasks,
            client,
            to_handler,
        })
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

    /// Spawns a task to send a motion profile request.
    ///
    /// The response will eventually arrive in [`EventHandler::next`].
    pub fn send_motion_profile_request(&mut self, request: motion_profile::Request) {
        let client = self.client.clone();
        let to_handler = self.to_handler.clone();

        tokio::spawn(async move {
            match client.send_resp::<MotionRequestEndpoint>(&request).await {
                Ok(response) => {
                    to_handler.send(Ok(TuiEvent::MCU(MCUEvent::MotionProfileRequestResponse(
                        Response::new(response, Local::now().time()),
                    ))))
                }
                Err(wire_err) => {
                    to_handler.send(Err(eyre!("Failed to send command: {}", wire_err)))
                }
            }
        });
    }

    /// Notifies the MCU that the app is closing.
    ///
    /// Although this method usually finishes immediately, it times out after 1 second.
    pub async fn send_disconnect_notification(&mut self) {
        let _ = timeout(
            Duration::from_secs(1),
            self.client
                .publish::<HostDisconnecting>(INITIAL_VAR_SEQ, &()),
        )
        .await;
    }
    /// Spawns a task to send a vacuum pump request.
    ///
    /// The response will eventually arrive in [`EventHandler::next`].
    pub fn send_vacuum_pump_request(&mut self, request: vacuum_pump::Request) {
        let client = self.client.clone();
        let to_handler = self.to_handler.clone();

        tokio::spawn(async move {
            match client
                .send_resp::<VacuumPumpRequestEndpoint>(&request)
                .await
            {
                Ok(()) => to_handler.send(Ok(TuiEvent::MCU(MCUEvent::VacuumPumpRequestResponse))),
                Err(wire_err) => {
                    to_handler.send(Err(eyre!("Failed to send command: {}", wire_err)))
                }
            }
        });
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

/// Awaits messages from a subscription in a loop, and forwards them to the handler.
async fn await_messages<S>(
    mut subscription: Subscription<S>,
    to_handler: UnboundedSender<Result<TuiEvent>>,
) where
    S: DeserializeOwned + Into<MCUEvent>,
{
    // As soon as the stream closes, the terminal must close as well.
    while let Some(state) = subscription.recv().await {
        if to_handler.send(Ok(TuiEvent::MCU(state.into()))).is_err() {
            break;
        }
    }
    let _ = to_handler.send(Err(eyre!("postcard_rpc closed a stream")));
}

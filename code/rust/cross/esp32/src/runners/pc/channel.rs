//! This module contains the channel functionality for the PC runner.

use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    mutex::Mutex,
    zerocopy_channel::{Channel, Receiver, Sender},
};
use sc_messages::{icd::McuMessage, motion_profile};
use static_cell::{ConstStaticCell, StaticCell};

use crate::gpio::pwm::SETPOINT_LIST_LENGTH;

/// The buffer used by [`REQUEST_CHANNEL`].
pub static REQUEST_BUFFER: ConstStaticCell<[motion_profile::HostMessage; SETPOINT_LIST_LENGTH]> =
    ConstStaticCell::new([const { motion_profile::HostMessage::Stop }; _]);

/// Used for passing [`motion_profile::HostMessage`]s from the server.
///
/// This is zerocopy because the messages are expensive to copy.
/// This uses [`NoopRawMutex`] because data is only shared in one executor.
pub static REQUEST_CHANNEL: StaticCell<Channel<NoopRawMutex, motion_profile::HostMessage>> =
    StaticCell::new();

pub type RunnerRequestReceiver = Receiver<'static, NoopRawMutex, motion_profile::HostMessage>;
pub type RunnerRequestSender = Sender<'static, NoopRawMutex, motion_profile::HostMessage>;

/// The buffer used by [`RESPONSE_CHANNEL`].
pub static RESPONSE_BUFFER: ConstStaticCell<[McuMessage; 4]> =
    ConstStaticCell::new([const { McuMessage::Heartbeat }; _]);

/// Used for passing [`McuMessage`]s to the server.
///
/// This is zerocopy because the messages are expensive to copy.
/// This uses [`NoopRawMutex`] because data is only shared in one executor.
pub static RESPONSE_CHANNEL: StaticCell<Channel<NoopRawMutex, McuMessage>> = StaticCell::new();

pub type ResponseReceiver = Receiver<'static, NoopRawMutex, McuMessage>;
pub type ResponseSender = Sender<'static, NoopRawMutex, McuMessage>;

/// The mutex for sharing the [`ResponseSender`].
///
/// A mutex is necessary because zerocopy channels are single-producer only,
/// so the sender must be shared.
pub type ResponseSenderMutex = Mutex<NoopRawMutex, ResponseSender>;

/// The static cell for storing [`ResponseSenderMutex`].
pub static RESPONSE_SENDER_MUTEX: StaticCell<ResponseSenderMutex> = StaticCell::new();

//! The [interface control document](https://en.wikipedia.org/wiki/Interface_control_document) for the microcontrollers and host PC.
use postcard_rpc::{TopicDirection, endpoints, topics};

use crate::{
    commands::{Command, CommandResult},
    motion_profile::State,
};

/// The baud rate for UART communication.
///
/// This value was taken from [`esp_hal::uart::Config::default`]
/// and is placed here so [`esp_hal::uart::Config::default`] doesn't change it under our feet.
pub const BAUD_RATE: u32 = 115_200;

endpoints! {
    list = ENDPOINTS_LIST;
    | EndpointTy      | RequestTy | ResponseTy    | Path                |
    |-----------------|-----------|---------------|---------------------|
    | CommandEndpoint | Command   | CommandResult | "endpoints/command" |
}

topics! {
   list = TOPICS_LIST;
   direction = TopicDirection::ToClient;
   | TopicTy                 | MessageTy | Path                          |
   |-------------------------|-----------|-------------------------------|
   | MotionProfileStateTopic | State     | "topics/motion_profile/state" |
}

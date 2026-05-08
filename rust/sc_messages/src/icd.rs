//! The [interface control document](https://en.wikipedia.org/wiki/Interface_control_document) for the microcontrollers and host PC.
use postcard_rpc::{TopicDirection, endpoints, topics};

use crate::{
    motion_profile::{Request as MotionProfileRequest, RequestResult, StateOrDisabled},
    vacuum_pump::Request as VacuumPumpRequest,
};

/// The baud rate for UART communication.
///
/// This value was taken from [`esp_hal::uart::Config::default`]
/// and is placed here so [`esp_hal::uart::Config::default`] doesn't change it under our feet.
pub const BAUD_RATE: u32 = 115_200;

endpoints! {
    list = ENDPOINTS_LIST;
    | EndpointTy      | RequestTy     | ResponseTy    | Path                |
    |-----------------|---------------|---------------|---------------------|
    | MotionRequestEndpoint | MotionProfileRequest | RequestResult | "endpoints/motion_profile/Request" |
    | VacuumPumpRequestEndpoint | VacuumPumpRequest | () | "endpoints/vacuum_pump/Request" |
}

topics! {
    list = TOPICS_TO_SERVER_LIST;
    direction = TopicDirection::ToServer;
    | TopicTy                 | MessageTy       | Path                          |
    |-------------------------|-----------------|-------------------------------|
    | HostDisconnecting | () | "topics/host/disconnecting" |
}

topics! {
   list = TOPICS_TO_CLIENT_LIST;
   direction = TopicDirection::ToClient;
   | TopicTy                 | MessageTy       | Path                          |
   |-------------------------|-----------------|-------------------------------|
   | MotionProfileStateTopic | StateOrDisabled | "topics/motion_profile/state" |
}

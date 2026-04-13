//! The [interface control document](https://en.wikipedia.org/wiki/Interface_control_document) for the microcontrollers and host PC.
use postcard_rpc::{TopicDirection, endpoints, topics};

use crate::{
    commands::{Command, CommandResult},
    motion_profile::State,
};

endpoints! {
    list = ENDPOINTS_LIST;
    | EndpointTy      | RequestTy | ResponseTy    | Path                |
    |-----------------|-----------|---------------|---------------------|
    | CommandEndpoint | Command   | CommandResult | "endpoints/command" |
}

topics! {
   list = TOPICS_LIST;
   direction = TopicDirection::ToClient;
   | TopicTy            | MessageTy | Path                          |
   |--------------------|-----------|-------------------------------|
   | MotionProfileState | State     | "topics/motion_profile/state" |
}

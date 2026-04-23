use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex},
    channel::Sender,
};
use esp_hal::{
    Async,
    gpio::Output,
    uart::{UartRx, UartTx},
};
use postcard_rpc::{
    define_dispatch,
    header::{VarHeader, VarSeq},
    server::impls::embedded_io_async_v0_6::{EioWireRx, EioWireSpawn, EioWireTx, WireStorage},
};
use sc_messages::{
    icd::{ENDPOINTS_LIST, MotionRequestEndpoint, TOPICS_LIST, VacuumPumpRequestEndpoint},
    motion_profile::{self, RequestRefused},
    vacuum_pump,
};
use static_cell::ConstStaticCell;

use crate::{REQUEST_CHANNEL_LENGTH, REQUEST_RESPONSE_SIGNAL};

/// The size of the buffers used by postcard-rpc.
pub const BUFFER_SIZE: usize = 1024;

/// The buffer used for receiving frames.
pub static FRAME_BUFFER: ConstStaticCell<[u8; BUFFER_SIZE]> =
    ConstStaticCell::new([0; BUFFER_SIZE]);

/// The storage that provides wire Tx and Rx.
pub static WIRE_STORAGE: WireStorage<
    UartRx<'static, Async>,
    UartTx<'static, Async>,
    CriticalSectionRawMutex,
    BUFFER_SIZE,
    BUFFER_SIZE,
> = WireStorage::new();

/// According to [the example](https://github.com/jamesmunns/postcard-rpc/blob/17dc2360a21c5caad5a20efb6a0a276df29ec945/example/firmware/src/bin/comms-02.rs#L277),
/// publish requires 0.
pub const SEQUENCE_NUMBER: VarSeq = VarSeq::Seq1(0);

pub type WireTx = EioWireTx<CriticalSectionRawMutex, UartTx<'static, Async>>;

pub type WireRx = EioWireRx<UartRx<'static, Async>>;

/// Information shared to all handlers.
pub struct Context {
    /// Used to pass the commands to the runner.
    to_runner: Sender<'static, NoopRawMutex, motion_profile::Request, REQUEST_CHANNEL_LENGTH>,
    /// Used to control the vacuum pump.
    vacuum_pump_pin: Output<'static>,
}

impl Context {
    /// Initializes the context.
    #[must_use]
    pub fn new(
        to_runner: Sender<'static, NoopRawMutex, motion_profile::Request, REQUEST_CHANNEL_LENGTH>,
        vacuum_pump_pin: Output<'static>,
    ) -> Self {
        Self {
            to_runner,
            vacuum_pump_pin,
        }
    }
}

/// Handles receiving requests from the host PC,
/// forwarding them to the motion profile runner,
/// and returning the command reponse.
async fn handle_motion_profile_request(
    context: &mut Context,
    _: VarHeader,
    request: motion_profile::Request,
) -> Result<(), RequestRefused> {
    context.to_runner.send(request).await;
    REQUEST_RESPONSE_SIGNAL.wait().await
}

/// Handles vacuum pump requests immediately.
#[allow(
    clippy::needless_pass_by_value,
    reason = "request is cheaper to pass by value than by reference."
)]
fn handle_vacuum_pump_request(context: &mut Context, _: VarHeader, request: vacuum_pump::Request) {
    match request {
        vacuum_pump::Request::Enable => context.vacuum_pump_pin.set_high(),
        vacuum_pump::Request::Disable => context.vacuum_pump_pin.set_low(),
    }
}

define_dispatch! {
    app: Dispatcher;
    spawn_fn: spawn_fn;
    tx_impl: EioWireTx<CriticalSectionRawMutex, UartTx<'static, Async>>;
    spawn_impl: EioWireSpawn;
    context: Context;

    endpoints: {
        list: ENDPOINTS_LIST;

        | EndpointTy      | kind  | handler         |
        |-----------------|-------|-----------------|
        | MotionRequestEndpoint | async | handle_motion_profile_request |
        | VacuumPumpRequestEndpoint | blocking | handle_vacuum_pump_request |
    };

    topics_in: {
        list: TOPICS_LIST;

        | TopicTy            | kind  | handler              |
        |--------------------|-------|----------------------|
    };

    topics_out: {
        list: TOPICS_LIST;
    };
}

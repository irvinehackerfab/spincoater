use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex},
    zerocopy_channel::Sender,
};
use esp_hal::{
    Async,
    uart::{UartRx, UartTx},
};
use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    server::impls::embedded_io_async_v0_6::{EioWireRx, EioWireSpawn, EioWireTx, WireStorage},
};
use sc_messages::{
    commands::{Command, CommandRefused},
    icd::{CommandEndpoint, ENDPOINTS_LIST, TOPICS_LIST},
};
use static_cell::ConstStaticCell;

use crate::COMMAND_RESPONSE_SIGNAL;

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

pub type WireTx = EioWireTx<CriticalSectionRawMutex, UartTx<'static, Async>>;

pub type WireRx = EioWireRx<UartRx<'static, Async>>;

/// Information shared to all handlers.
pub struct Context {
    /// Used to pass the commands to the runner.
    to_runner: Sender<'static, NoopRawMutex, Command>,
}

impl Context {
    /// Initializes the context.
    #[must_use]
    pub fn new(to_runner: Sender<'static, NoopRawMutex, Command>) -> Self {
        Self { to_runner }
    }
}

/// Handles receiving commands from the host PC,
/// forwarding them to the motion profile runner,
/// and returning the command reponse.
async fn command_handler(
    context: &mut Context,
    _: VarHeader,
    command: Command,
) -> Result<(), CommandRefused> {
    let buf = context.to_runner.send().await;
    *buf = command;
    context.to_runner.send_done();
    COMMAND_RESPONSE_SIGNAL.wait().await
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
        | CommandEndpoint | async | command_handler |
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

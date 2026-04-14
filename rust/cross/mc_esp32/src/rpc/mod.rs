use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use esp_hal::Async;
use esp_hal::uart::{UartRx, UartTx};
use postcard_rpc::define_dispatch;
use postcard_rpc::header::VarHeader;
use postcard_rpc::server::Sender;
use postcard_rpc::server::impls::embedded_io_async_v0_6::WireStorage;
use postcard_rpc::server::impls::embedded_io_async_v0_6::{EioWireSpawn, EioWireTx};
use sc_messages::commands::{Command, CommandRefused};
use sc_messages::icd::{CommandEndpoint, ENDPOINTS_LIST, MotionProfileState, TOPICS_LIST};
use sc_messages::motion_profile;
use static_cell::ConstStaticCell;

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

/// Information shared to all handlers.
pub struct Context {}

async fn command_handler(
    context: &mut Context,
    _: VarHeader,
    command: Command,
) -> Result<(), CommandRefused> {
    todo!()
}

async fn motion_profile_state(
    context: &mut Context,
    _: VarHeader,
    state: motion_profile::State,
    sender: &Sender<EioWireTx<CriticalSectionRawMutex, UartTx<'static, Async>>>,
) {
    todo!()
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
        | MotionProfileState | async | motion_profile_state |
    };

    topics_out: {
        list: TOPICS_LIST;
    };
}

//! This module contains all of the wifi functionality.
pub mod channel;
pub mod tcp;

use core::{
    fmt::{Debug, Display},
    net::Ipv4Addr,
};
use embassy_net::{IpListenEndpoint, Runner, StackResources};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use esp_radio::{
    Controller,
    wifi::{AuthMethod, WifiController, WifiDevice, WifiEvent},
};
use static_cell::StaticCell;

use crate::gpio::display::terminal::channel::{
    TERMINAL_CHANNEL_SIZE, TuiEvent, send_event_or_report,
};

/// Keep this up to date with the address listed in `../../host_tui/src/main.rs`
pub const GATEWAY_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 2, 1);
/// Keep this up to date with the address listed in `../../host_tui/src/main.rs`
pub const PORT: u16 = 8080;
/// We want to listen for connections no matter the IP address.
pub const IP_LISTEN_ENDPOINT: IpListenEndpoint = IpListenEndpoint {
    addr: None,
    port: PORT,
};

/// I would use [`AuthMethod::Wpa3Personal`], but it isn't supported for access point mode.
pub const AUTH_METHOD: AuthMethod = AuthMethod::Wpa2Personal;
/// We only want one active connection at a time.
pub const MAX_CONNECTIONS: u16 = 1;

/// The static variable that holds the radio.
pub static RADIO: StaticCell<Controller> = StaticCell::new();
/// We only use 1 socket right now.
pub static STACK_RESOURCES: StaticCell<StackResources<1>> = StaticCell::new();

/// All possible access point states.
#[derive(Debug, Default, Clone, Copy)]
pub enum ApState {
    #[default]
    /// The access point is waiting for a connection.
    Disconnected,
    /// The host is connected to the access point.
    Connected,
}

/// Display implementation used by [`crate::gpio::display::terminal::TerminalState::draw`].
impl Display for ApState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Access point state: {}",
            match self {
                ApState::Disconnected => "no connection",
                ApState::Connected => "connected",
            }
        )
    }
}

/// This task detects various events emitted by the wifi controller.
#[embassy_executor::task]
pub async fn handle_connections(
    mut wifi_controller: WifiController<'static>,
    to_terminal: Sender<'static, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
) -> ! {
    // No need to send [`ApClientDisconnected`] because the terminal starts with that state.
    loop {
        wifi_controller
            .wait_for_event(WifiEvent::ApStaConnected)
            .await;
        send_event_or_report(&to_terminal, TuiEvent::WifiEvent(ApState::Connected)).await;
        wifi_controller
            .wait_for_event(WifiEvent::ApStaDisconnected)
            .await;
        send_event_or_report(&to_terminal, TuiEvent::WifiEvent(ApState::Disconnected)).await;
    }
}

/// This task runs the network stack.
#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) -> ! {
    runner.run().await;
}

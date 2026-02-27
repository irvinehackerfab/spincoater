//! This module contains all of the wifi functionality.
pub mod tcp;

use core::net::Ipv4Addr;
use embassy_net::{IpListenEndpoint, Runner, StackResources};
use embassy_time::Timer;
use esp_println::println;
use esp_radio::{
    Controller,
    wifi::{AuthMethod, WifiController, WifiDevice, WifiEvent},
};
use static_cell::StaticCell;

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

/// This task restarts the wifi 5 seconds after it stops.
#[embassy_executor::task]
pub async fn controller_task(mut wifi_controller: WifiController<'static>) {
    println!("Wifi: starting connection loop");
    loop {
        match wifi_controller.start_async().await {
            Ok(()) => {
                println!("Wifi: Access point started");
                wifi_controller.wait_for_event(WifiEvent::ApStop).await;
                println!("Wifi: Access point stopped");
            }
            Err(err) => {
                println!("Wifi: Error when starting controller: {}", err);
            }
        }
        println!("Wifi: Waiting 5 seconds before restarting...");
        Timer::after_secs(5).await;
    }
}

/// This task runs the network stack.
#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}

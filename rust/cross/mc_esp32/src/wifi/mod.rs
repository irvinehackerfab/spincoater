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

pub const GATEWAY_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 2, 1);
pub const PORT: u16 = 8080;
pub const IP_LISTEN_ENDPOINT: IpListenEndpoint = IpListenEndpoint {
    addr: None,
    port: PORT,
};

pub const AUTH_METHOD: AuthMethod = AuthMethod::Wpa2Personal;
pub const MAX_CONNECTIONS: u16 = 1;

// Static resources
/// The radio must be made static so Rust doesn't think it can ever be dropped.
pub static RADIO: StaticCell<Controller> = StaticCell::new();
/// We only use 1 socket right now
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

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}

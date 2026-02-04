pub mod error;

use core::net::Ipv4Addr;
use embassy_net::{IpListenEndpoint, Runner, StackResources, tcp::TcpSocket};
use embassy_time::Timer;
use esp_println::println;
use esp_radio::{
    Controller,
    wifi::{AuthMethod, WifiController, WifiDevice, WifiEvent},
};
use postcard::{self, Error};
use sc_messages::Message;
use static_cell::StaticCell;

use crate::tcp::error::ReadError;

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
// The config can be made static to avoid String reallocation upon wifi restart.
// pub(crate) static WIFI_CONFIG: StaticCell<ModeConfig> = StaticCell::new();

pub const BUFFER_SIZE: usize = 64;
pub static RX_BUFFER: StaticCell<[u8; BUFFER_SIZE]> = StaticCell::new();
pub static TX_BUFFER: StaticCell<[u8; BUFFER_SIZE]> = StaticCell::new();

/// This task restarts the wifi 5 seconds after it stops.
#[embassy_executor::task]
pub async fn controller_task(mut wifi_controller: WifiController<'static>) {
    println!("starting connection loop");
    loop {
        match wifi_controller.start_async().await {
            Ok(()) => {
                println!("Access point started");
                wifi_controller.wait_for_event(WifiEvent::ApStop).await;
                println!("Access point stopped");
            }
            Err(err) => {
                println!("Error when starting wifi: {}", err);
            }
        }
        println!("Waiting 5 seconds before restarting...");
        Timer::after_secs(5).await;
    }
}

/// Reads the transmit buffer repeatedly until a complete message is found or an error occurs.
///
/// The message must be [COBS encoded](https://docs.rs/postcard/latest/postcard/ser_flavors/struct.Cobs.html)
/// and must fit in [BUFFER_SIZE] bytes.
pub async fn recv_message<'a>(socket: &mut TcpSocket<'a>) -> Result<Message, ReadError> {
    loop {
        // BUFFER_SIZE is too small if we're filling up the buffer.
        assert!(socket.recv_queue() < BUFFER_SIZE);
        if let Some(message) = socket
            .read_with(|written_chunk| {
                // O(n) search is worth it because it eliminates the need for copying the buffer.
                match written_chunk.contains(&0u8) {
                    true => {
                        // Attempt to deserialize.
                        let deserialization_result =
                            postcard::from_bytes_cobs::<Message>(written_chunk);
                        // Wraps the message in an Option if there is a message.
                        let resulting_option = deserialization_result.map(Option::from);
                        // Tell the socket to clear the bytes we used and return our result.
                        (written_chunk.len(), resulting_option)
                    }
                    // Do nothing and try again next time.
                    false => (0, Ok(None)),
                }
            })
            .await??
        {
            return Ok(message);
        }
    }
}

/// Uses the transmit buffer repeatedly until a complete message can be sent or an error occurs.
///
/// The message is [COBS encoded](https://docs.rs/postcard/latest/postcard/ser_flavors/struct.Cobs.html).
/// The message must fit in [BUFFER_SIZE] bytes.
pub async fn send_message<'a>(
    message: Message,
    socket: &mut TcpSocket<'a>,
) -> Result<(), ReadError> {
    loop {
        // BUFFER_SIZE is too small if we're filling up the buffer.
        assert!(socket.send_queue() < BUFFER_SIZE);
        if socket
            .write_with(
                |empty_chunk| match postcard::to_slice_cobs(&message, empty_chunk) {
                    // The message has been written to the buffer, so let the socket send it.
                    Ok(written_chunk) => (written_chunk.len(), Ok(true)),
                    // There isn't enough space for the message yet, so try again next time.
                    Err(Error::SerializeBufferFull) => (0, Ok(false)),
                    // A serialization error occurred so give up and return the error.
                    Err(err) => (0, Err(err)),
                },
            )
            .await??
        {
            return Ok(());
        }
    }
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}

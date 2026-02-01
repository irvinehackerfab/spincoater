pub mod error;

use core::net::Ipv4Addr;
use embassy_net::{IpListenEndpoint, Runner, StackResources, tcp::TcpSocket};
use embassy_time::Timer;
use embedded_io_async::Write;
use esp_println::println;
use esp_radio::{
    Controller,
    wifi::{AuthMethod, WifiController, WifiDevice, WifiEvent},
};
use heapless::Vec;
use postcard::{self, Error};
use sc_messages::Message;
use static_cell::StaticCell;
use zeroize::Zeroize;

use crate::wifi::error::ReadError;

pub(crate) const GATEWAY_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 2, 1);
pub(crate) const PORT: u16 = 8080;
pub(crate) const IP_LISTEN_ENDPOINT: IpListenEndpoint = IpListenEndpoint {
    addr: None,
    port: PORT,
};

// Do not hardcode sensitive information like this.
// Instead, pass in the variables as environment variables when you compile, like this:
// SSID=_ PASSWORD=_ cargo run --release
pub(crate) const SSID: &str = env!("SSID");
pub(crate) const PASSWORD: &str = env!("PASSWORD");
pub(crate) const AUTH_METHOD: AuthMethod = AuthMethod::Wpa3Personal;
pub(crate) const MAX_CONNECTIONS: u16 = 1;

// Static resources
/// The radio must be made static so Rust doesn't think it can ever be dropped.
pub(crate) static RADIO: StaticCell<Controller> = StaticCell::new();
/// We only use 1 socket right now
pub(crate) static STACK_RESOURCES: StaticCell<StackResources<1>> = StaticCell::new();
// The config can be made static to avoid String reallocation upon wifi restart.
// pub(crate) static WIFI_CONFIG: StaticCell<ModeConfig> = StaticCell::new();

pub(crate) const BUFFER_SIZE: usize = 64;
pub(crate) static RX_BUFFER: StaticCell<Vec<u8, BUFFER_SIZE>> = StaticCell::new();
pub(crate) static TX_BUFFER: StaticCell<Vec<u8, BUFFER_SIZE>> = StaticCell::new();
pub(crate) static READ_BUFFER: StaticCell<Vec<u8, BUFFER_SIZE>> = StaticCell::new();

/// This task restarts the wifi 5 seconds after it stops.
#[embassy_executor::task]
pub(crate) async fn controller_task(mut wifi_controller: WifiController<'static>) {
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

pub(crate) async fn recv_message<'a>(
    socket: &mut TcpSocket<'a>,
    buffer: &mut Vec<u8, BUFFER_SIZE>,
) -> Result<Message, ReadError> {
    // Position in the buffer for the next read to start from.
    let mut position = 0;
    let result = loop {
        if position >= buffer.len() {
            panic!(
                "Not enough space in the buffer to receive the message. Please increase BUFFER_SIZE."
            )
        }
        match socket.read(&mut buffer[position..]).await {
            Ok(0) => break Err(ReadError::SocketClosed),
            Ok(len) => {
                // Read up to the end of the written segment.
                match postcard::from_bytes::<Message>(&buffer[..(position + len)]) {
                    Ok(message) => {
                        break Ok(message);
                    }
                    // Case: There is more to read, so update position and keep reading.
                    Err(Error::DeserializeUnexpectedEnd) => position += len,
                    Err(err) => break Err(err.into()),
                }
            }
            Err(err) => break Err(err.into()),
        }
    };
    buffer.zeroize();
    result
}

pub(crate) async fn send_message<'a>(
    message: Message,
    socket: &mut TcpSocket<'a>,
    buffer: &mut Vec<u8, BUFFER_SIZE>,
) -> Result<(), ReadError> {
    let written_chunk = postcard::to_slice(&message, buffer).inspect_err(|err| {
        if let Error::SerializeBufferFull = err {
            panic!(
                "Not enough space in the buffer to send the message. Please increase BUFFER_SIZE."
            )
        }
    })?;
    socket.write_all(written_chunk).await?;
    buffer.zeroize();
    Ok(())
}

#[embassy_executor::task]
pub(crate) async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}

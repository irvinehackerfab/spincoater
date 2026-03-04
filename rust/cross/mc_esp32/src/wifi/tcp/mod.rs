//! This module contains all of the TCP-socket-specific functionality of the wifi.
pub mod error;

use core::fmt::Display;

use embassy_futures::select::select;
use embassy_net::tcp::{Error, TcpReader, TcpSocket, TcpWriter};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Receiver, Sender},
};
use embassy_time::Duration;
use esp_println::println;
use sc_messages::Message;
use static_cell::ConstStaticCell;

use crate::{
    gpio::display::terminal::channel::{
        ChannelKind, TERMINAL_CHANNEL_SIZE, TuiEvent, send_event_or_report,
    },
    wifi::{
        IP_LISTEN_ENDPOINT,
        channel::{HANDLER_CHANNEL_SIZE, send_msg_or_report},
        tcp::error::TcpError,
    },
};

/// How long the MCU will wait before disconnecting an inactive host device from both
/// the access point and socket.
pub const TIMEOUT: Duration = Duration::from_secs(10);
/// How often the MCU will send keep-alive packets.
/// This prevents the socket from closing due to inactivity.
pub const KEEP_ALIVE: Duration = Duration::from_secs(5);

/// The number of bytes each buffer can hold.
/// This should be enough bytes to store multiple [`sc_messages::Message`]s.
///
/// Keep this up to date with `../../sc_messages/src/lib.rs` `BUFFER_SIZE`
pub const BUFFER_SIZE: usize = 64;
/// The static variable for the receive buffer.
pub static RX_BUFFER: ConstStaticCell<[u8; BUFFER_SIZE]> = ConstStaticCell::new([0u8; BUFFER_SIZE]);
/// The static variable for the transmit buffer.
pub static TX_BUFFER: ConstStaticCell<[u8; BUFFER_SIZE]> = ConstStaticCell::new([0u8; BUFFER_SIZE]);

/// All possible states for the socket.
#[derive(Debug, Default)]
pub enum SocketState {
    /// The host is not connected to the socket.
    #[default]
    Disconnected,
    /// The host is connected to the socket.
    Connected,
}

/// Display implementation used by [`crate::gpio::display::terminal::TerminalState::draw`].
impl Display for SocketState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Socket state: {}",
            match self {
                SocketState::Disconnected => "no connection",
                SocketState::Connected => "connected",
            }
        )
    }
}

/// Reads the transmit buffer repeatedly until a complete message is found or an error occurs.
///
/// The message must be [COBS encoded](https://docs.rs/postcard/latest/postcard/ser_flavors/struct.Cobs.html)
/// and must fit in [`BUFFER_SIZE`] bytes.
///
/// # Errors
/// Returns an error if deserialization or reading fails.
///
/// # Panics
/// Panics if the socket's receive buffer has [`BUFFER_SIZE`] or more bytes queued.
pub async fn recv_message(reader: &mut TcpReader<'_>) -> Result<Message, TcpError> {
    loop {
        // BUFFER_SIZE is too small if we're filling up the buffer.
        assert!(reader.recv_queue() < BUFFER_SIZE);
        if let Some(message) = reader
            .read_with(|written_chunk| {
                // Only deserialize if at least one complete message is in the buffer.
                match written_chunk.iter().position(|byte| *byte == 0u8) {
                    Some(idx) => {
                        // Get the actual number of bytes to read.
                        let end = idx + 1;
                        // Attempt to deserialize once.
                        let deserialization_result =
                            postcard::from_bytes_cobs::<Message>(&mut written_chunk[..end]);
                        // Wraps the message in an Option if there is a message.
                        let resulting_option = deserialization_result.map(Option::from);
                        // Tell the socket to clear the bytes we used and return our result.
                        (end, resulting_option)
                    }
                    None => (0, Ok(None)),
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
/// The message must fit in [`BUFFER_SIZE`] bytes or else this method will never return.
///
/// # Errors
/// Returns an error if serialization or writing fails.
pub async fn send_message(message: Message, writer: &mut TcpWriter<'_>) -> Result<(), TcpError> {
    loop {
        if writer
            .write_with(
                |empty_chunk| match postcard::to_slice_cobs(&message, empty_chunk) {
                    // The message has been written to the buffer, so let the socket send it.
                    Ok(written_chunk) => (written_chunk.len(), Ok(true)),
                    // There isn't enough space for the message yet, so try again next time.
                    Err(postcard::Error::SerializeBufferFull) => (0, Ok(false)),
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

/// Receives messages from the host device, and sends them to the message handler.
///
/// This loop continues until an error occurs.
/// # Errors
/// See [`TcpError`] for all possible errors.
pub async fn receive_unhandled_messages(
    reader: &mut TcpReader<'_>,
    to_msg_handler: &Sender<'_, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
    to_terminal: &Sender<'_, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
) {
    loop {
        match recv_message(reader).await {
            Ok(message) => {
                send_msg_or_report(to_msg_handler, message, to_terminal, ChannelKind::RecvMsg)
                    .await;
            }
            Err(err) => {
                println!("RX error: {err:?}");
                break;
            }
        }
    }
}

/// Receives messages that have been handled by the handler, and sends them to the host device.
///
/// This loop continues until an error occurs.
/// # Errors
/// See [`TcpError`] for all possible errors.
pub async fn announce_handled_messages(
    writer: &mut TcpWriter<'_>,
    from_msg_handler: &Receiver<'_, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
) {
    loop {
        let message = from_msg_handler.receive().await;
        if let Err(err) = send_message(message, writer).await {
            println!("TX error: {err:?}");
            break;
        }
    }
}

/// This function waits for connections, and then handles sending and receiving messages using the provided channels.
/// Upon disconnect, it waits for the next connection.
///
/// # Panics
/// This function panics if it contains a logic error that needs to be fixed.
pub async fn handle_socket_connections(
    mut socket: TcpSocket<'_>,
    to_msg_handler: Sender<'_, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
    from_msg_handler: Receiver<'_, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
    to_terminal: Sender<'_, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
) -> ! {
    loop {
        socket
            .accept(IP_LISTEN_ENDPOINT)
            .await
            .expect("Failed to listen for socket connections");
        send_event_or_report(&to_terminal, TuiEvent::SocketEvent(SocketState::Connected)).await;
        let (mut reader, mut writer) = socket.split();
        // Cancel receiving and transmitting as soon as an error occurs.
        // This gives the socket the opportunity to abort.
        select(
            receive_unhandled_messages(&mut reader, &to_msg_handler, &to_terminal),
            announce_handled_messages(&mut writer, &from_msg_handler),
        )
        .await;
        socket.abort();
        let _ = socket.flush().await;
        send_event_or_report(
            &to_terminal,
            TuiEvent::SocketEvent(SocketState::Disconnected),
        )
        .await;
        // Flush all data from the receive buffer as well.
        // Embassy may prevent this from working properly,
        // in which case we must wait for a new connection to flush the buffer.
        flush_rx_buffer(&mut socket)
            .await
            .expect("Failed to flush RX buffer");
    }
}

/// Flushes the receive buffer of the socket if there is data left in it.
///
/// # Errors
/// This function returns an error if Embassy disallows flushing the receive buffer.
/// If this happens, try calling this function after a new connection has been established instead.
pub async fn flush_rx_buffer(socket: &mut TcpSocket<'_>) -> Result<(), Error> {
    if socket.may_recv() {
        socket.read_with(|bytes| (bytes.len(), ())).await?;
    }
    Ok(())
}

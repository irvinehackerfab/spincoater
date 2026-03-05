//! This module contains all of the TCP-socket-specific functionality of the wifi.
pub mod error;

use core::fmt::Display;

use bytes::BytesMut;
use embassy_futures::select::select;
use embassy_net::tcp::{TcpReader, TcpSocket, TcpWriter};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Receiver, Sender},
};
use embassy_time::Duration;
use esp_println::println;
use postcard::from_bytes_cobs;
use sc_messages::{Message, STOP_DUTY};
use static_cell::{ConstStaticCell, StaticCell};

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

/// Because smoltcp (and therefore [`embassy_net`]) does not always allow you to read all bytes
/// in [`RX_BUFFER`] at once, we need another buffer to store the result of multiple [`TcpReader::read_with`] calls.
pub static RX_BUFFER_2: StaticCell<BytesMut> = StaticCell::new();

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

/// Uses the transmit buffer repeatedly until a complete message can be sent or an error occurs.
///
/// The message is [COBS encoded](https://docs.rs/postcard/latest/postcard/ser_flavors/struct.Cobs.html).
///
/// The message must fit in [`BUFFER_SIZE`] bytes or else this method will never return,
/// so keep an eye on the size of [`Message`].
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
/// Deserialization errors are printed out.
/// All errors are printed out and cause
/// [`TuiEvent::SocketEvent`] with [`SocketState::Disconnected`] to be sent to the terminal.
pub async fn receive_unhandled_messages(
    reader: &mut TcpReader<'_>,
    buffer: &mut BytesMut,
    to_msg_handler: &Sender<'_, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
    to_terminal: &Sender<'_, NoopRawMutex, TuiEvent, TERMINAL_CHANNEL_SIZE>,
) {
    'outer: while let Ok(()) = reader
        .read_with(|written_chunk| {
            buffer.extend_from_slice(written_chunk);
            (written_chunk.len(), ())
        })
        .await
    {
        let mut written_chunk = buffer.split();
        // We must search for 0 before deserializing because from_bytes_cobs mutates the slice regardless of success.
        while let Some(idx) = written_chunk.iter().position(|byte| *byte == 0u8) {
            let end = idx + 1;
            let mut msg_chunk = written_chunk.split_to(end);
            match from_bytes_cobs::<Message>(&mut msg_chunk) {
                Ok(message) => {
                    // Return message
                    send_msg_or_report(to_msg_handler, message, to_terminal, ChannelKind::RecvMsg)
                        .await;
                }
                Err(error) => {
                    // If deserialization fails, the task is done.
                    println!("Deserialization failed: {error}");
                    // Clear everything so the buffer can be reused.
                    written_chunk.unsplit(msg_chunk);
                    written_chunk.clear();
                    buffer.unsplit(written_chunk);
                    break 'outer;
                }
            }
            // Clear the deserialized data so we can search for another 0u8.
            msg_chunk.clear();
            written_chunk.unsplit(msg_chunk);
        }
        // We don't clear here because `read_with` may have given us half a message,
        // and will give the other half later.
        buffer.unsplit(written_chunk);
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
    buffer: &mut BytesMut,
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
            receive_unhandled_messages(&mut reader, buffer, &to_msg_handler, &to_terminal),
            announce_handled_messages(&mut writer, &from_msg_handler),
        )
        .await;
        // Clear the buffer of any unprocessed bytes.
        // Keep in mind that if `announce_handled_messages` cancels `receive_unhandled_messages`,
        // The buffer may have unprocessed bytes and may have lost some capacity that was split off.
        // This is okay, because BytesMut can reallocate.
        buffer.clear();
        // Abort the connection.
        socket.abort();
        let _ = socket.flush().await;
        // Update the TUI
        send_event_or_report(
            &to_terminal,
            TuiEvent::SocketEvent(SocketState::Disconnected),
        )
        .await;
        // Ensure PWM output is disabled
        send_msg_or_report(
            &to_msg_handler,
            Message::DutyCycle(STOP_DUTY),
            &to_terminal,
            ChannelKind::RecvMsg,
        )
        .await;
    }
}

pub mod error;

use embassy_futures::select::{Either, select};
use embassy_net::tcp::{Error, TcpReader, TcpSocket, TcpWriter};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::Duration;
use esp_println::println;
use sc_messages::Message;
use static_cell::{ConstStaticCell, StaticCell};

use crate::wifi::{IP_LISTEN_ENDPOINT, tcp::error::TcpError};

/// How long the MCU will wait before disconnecting the host device.
pub const TIMEOUT: Duration = Duration::from_secs(10);
/// How often the MCU will send keep-alive packets.
/// This prevents the socket from closing due to inactivity.
pub const KEEP_ALIVE: Duration = Duration::from_secs(5);

pub const BUFFER_SIZE: usize = 64;
pub static RX_BUFFER: StaticCell<[u8; BUFFER_SIZE]> = StaticCell::new();
pub static TX_BUFFER: StaticCell<[u8; BUFFER_SIZE]> = StaticCell::new();

/// The maximum number of messages allowed at a time in each channel to/from the message handler.
pub const HANDLER_CHANNEL_SIZE: usize = 2;
/// Used for passing messages from the socket to the message handler.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
/// This does not use a zerocopy channel because [`Message`] is currently smaller than a reference.
pub static RECV_MSG_CHANNEL: ConstStaticCell<Channel<NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>> =
    ConstStaticCell::new(Channel::new());

/// Used for passing messages from the message handler to the socket.
///
/// This uses `NoopRawMutex` because data is only shared in one executor.
/// This does not use a zerocopy channel because [`Message`] is currently smaller than a reference.
pub static SEND_MSG_CHANNEL: ConstStaticCell<Channel<NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>> =
    ConstStaticCell::new(Channel::new());

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
pub async fn send_message(message: &Message, writer: &mut TcpWriter<'_>) -> Result<(), TcpError> {
    loop {
        if writer
            .write_with(
                |empty_chunk| match postcard::to_slice_cobs(message, empty_chunk) {
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
    to_msg_handler: &mut Sender<'_, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
) -> TcpError {
    loop {
        match recv_message(reader).await {
            Ok(message) => {
                if to_msg_handler.try_send(message).is_err() {
                    println!(
                        "Receiver has no space to send the message. Please consider increasing HANDLER_CHANNEL_SIZE."
                    );
                    to_msg_handler.send(message).await;
                }
            }
            Err(err) => break err,
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
    from_msg_handler: &mut Receiver<'_, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
) -> TcpError {
    loop {
        let message = from_msg_handler.receive().await;
        if let Err(err) = send_message(&message, writer).await {
            break err;
        }
    }
}

/// This function waits for connections, and then handles sending and receiving messages using the provided channels.
/// Upon disconnect, it waits for the next connection.
///
/// # Panics
/// This function panics if it contains a logic error that needs to be fixed.
#[allow(
    clippy::large_stack_frames,
    reason = "printing is necessary for debugging."
)]
pub async fn handle_connections(
    mut socket: TcpSocket<'_>,
    mut to_msg_handler: Sender<'_, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
    mut from_msg_handler: Receiver<'_, NoopRawMutex, Message, HANDLER_CHANNEL_SIZE>,
) -> ! {
    loop {
        println!("Socket: Waiting for connection...");
        if let Err(err) = socket.accept(IP_LISTEN_ENDPOINT).await {
            println!("Wifi: Accept error: {:?}", err);
            continue;
        }
        println!(
            "Socket: Got connection from address {:?}",
            socket.remote_endpoint()
        );
        let (mut reader, mut writer) = socket.split();
        // Cancel receiving and transmitting as soon as an error occurs.
        // This gives the socket the opportunity to abort.
        match select(
            receive_unhandled_messages(&mut reader, &mut to_msg_handler),
            announce_handled_messages(&mut writer, &mut from_msg_handler),
        )
        .await
        {
            Either::First(err) => {
                println!("Receiver error: {err:?}");
            }
            Either::Second(err) => {
                println!("Transmitter error: {err:?}");
            }
        }
        socket.abort();
        let _ = socket.flush().await;
        // Flush all data from the receive buffer as well.
        flush_rx_buffer(&mut socket).await.expect(
            "socket has data in the receive buffer, but Embassy is preventing access to it",
        );
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

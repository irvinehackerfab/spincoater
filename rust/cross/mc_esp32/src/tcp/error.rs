use embassy_net::tcp::TcpSocket;
use esp_println::println;

pub enum ReadError {
    SocketClosed,
    TCPError(embassy_net::tcp::Error),
    PostcardError(postcard::Error),
}

impl ReadError {
    /// Prints errors out and aborts the TCP connection.
    pub async fn handle<'a>(self, socket: &mut TcpSocket<'a>) {
        match self {
            ReadError::SocketClosed => println!("Socket closed by peer."),
            ReadError::TCPError(error) => println!("TCP error: {error}."),
            ReadError::PostcardError(error) => {
                println!("Postcard error: {error}. Closing connection.");
            }
        }
        // This may not be necessary in every case, but I still need to test this.
        socket.abort();
        let _ = socket.flush().await;
    }
}

impl From<postcard::Error> for ReadError {
    fn from(value: postcard::Error) -> Self {
        ReadError::PostcardError(value)
    }
}

impl From<embassy_net::tcp::Error> for ReadError {
    fn from(value: embassy_net::tcp::Error) -> Self {
        ReadError::TCPError(value)
    }
}

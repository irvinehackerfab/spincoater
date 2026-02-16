#[derive(Debug)]
pub enum TcpError {
    SocketClosed,
    TCPError(embassy_net::tcp::Error),
    PostcardError(postcard::Error),
}

impl From<postcard::Error> for TcpError {
    fn from(value: postcard::Error) -> Self {
        TcpError::PostcardError(value)
    }
}

impl From<embassy_net::tcp::Error> for TcpError {
    fn from(value: embassy_net::tcp::Error) -> Self {
        TcpError::TCPError(value)
    }
}

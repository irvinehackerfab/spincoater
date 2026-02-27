//! This module contains wifi errors.

/// All possible wifi errors.
#[derive(Debug)]
pub enum TcpError {
    /// An error returned by the Embassy's TCP socket.
    TCPError(embassy_net::tcp::Error),
    /// An error returned by postcard's serialization/deserialization.
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

pub(crate) enum ReadError {
    SocketClosed,
    ConnectionReset,
    DeserializeError(postcard::Error),
}

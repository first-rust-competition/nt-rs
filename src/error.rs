use nt_network::NTVersion;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to connect to server (Connection aborted)")]
    ConnectionAborted,
    #[error("Connected closed unexpectedly.")]
    BrokenPipe,
    #[error("Server does not support the desired protocol version. Supported version: {supported_version:?}")]
    UnsupportedProtocolVersion {
        supported_version: NTVersion,
    },
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[cfg(feature = "websocket")]
impl From<tokio_tungstenite::tungstenite::error::Error> for Error {
    fn from(err: tokio_tungstenite::tungstenite::error::Error) -> Self {
        Error::Other(err.into())
    }
}

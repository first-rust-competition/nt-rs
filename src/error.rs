use crate::proto;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error decoding text frame: {0}")]
    JSON(#[from] serde_json::Error),
    #[error("Error decoding binary frame: {0}")]
    NTBin(#[from] proto::prelude::DecodeError),
    #[error("Transport error: {0}")]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

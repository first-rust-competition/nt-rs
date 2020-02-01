use failure::Fail;
use nt_network::NTVersion;


#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Failed to connect to server (Connection aborted)")]
    ConnectionAborted,
    #[fail(display = "Connected closed unexpectedly.")]
    BrokenPipe,
    #[fail(display = "Server does not support the desired protocol version. Supported version: {:?}", supported_version)]
    UnsupportedProtocolVersion {
        supported_version: NTVersion,
    },
    #[fail(display = "IO error")]
    IO {
        #[cause] cause: std::io::Error
    },
    #[fail(display = "Other error")]
    Other {
        #[cause] cause: failure::Error,
    }
}

impl From<std::io::Error> for Error {
    fn from(cause: std::io::Error) -> Self {
        Error::IO { cause }
    }
}

impl From<failure::Error> for Error {
    fn from(cause: failure::Error) -> Self {
        Error::Other { cause }
    }
}
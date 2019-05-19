use crate::constants::MAGIC;

#[derive(Debug)]
pub enum Error {
    ParseError(ensicoin_serializer::Error),
    InvalidConnectionState(String),
    InvalidMagic(u32),
    IoError(std::io::Error),
    ChannelError,
    ServerTermination,
    NoResponse,
    TimerError(tokio_timer::Error),
    StreamError,
    DatabaseError(sled::Error),
    InvalidBlock,
    NotFound(String),
    PoisonedLock,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::StreamError => write!(f, "Tokio stream failed"),
            Error::TimerError(e) => write!(f, "Timer failed: {}", e),
            Error::NoResponse => write!(f, "No response to ping"),
            Error::ParseError(e) => write!(f, "Parse error: {}", e),
            Error::InvalidConnectionState(st) => {
                write!(f, "Connection is in invalid state: {}", st)
            }
            Error::IoError(e) => write!(f, "IoError: {}", e),
            Error::InvalidMagic(n) => write!(f, "Invalid magic, got {} expected {}", n, MAGIC),
            Error::ChannelError => write!(f, "Server channel failed"),
            Error::ServerTermination => write!(f, "Server terminated the connection"),
            Error::NotFound(r) => write!(f, "Resource not found: {}", r),
            Error::DatabaseError(e) => write!(f, "Database error: {}", e),
            Error::PoisonedLock => write!(f, "Poisoned lock"),
            Error::InvalidBlock => write!(f, "Invalid Block"),
        }
    }
}

impl From<ensicoin_serializer::Error> for Error {
    fn from(error: ensicoin_serializer::Error) -> Self {
        Error::ParseError(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<tokio_timer::Error> for Error {
    fn from(error: tokio_timer::Error) -> Self {
        Error::TimerError(error)
    }
}

impl From<sled::Error> for Error {
    fn from(error: sled::Error) -> Self {
        Error::DatabaseError(error)
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Error::PoisonedLock
    }
}

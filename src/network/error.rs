use super::connection::State;
use crate::constants::MAGIC;

#[derive(Debug)]
pub enum Error {
    ParseError(ensicoin_serializer::Error),
    InvalidState(State),
    InvalidMagic(u32),
    IoError(std::io::Error),
    ChannelReceiverError(std::sync::mpsc::RecvError),
    ChannelError,
    ServerTermination,
    NoResponse,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NoResponse => write!(f, "No response to ping"),
            Error::ParseError(e) => write!(f, "Parse error: {}", e),
            Error::InvalidState(st) => write!(f, "Connection is in invalid state: {}", st),
            Error::IoError(e) => write!(f, "IoError: {}", e),
            Error::InvalidMagic(n) => write!(f, "Invalid magic, got {} expected {}", n, MAGIC),
            Error::ChannelError => write!(f, "Server channel failed"),
            Error::ServerTermination => write!(f, "Server terminated the connection"),
            Error::ChannelReceiverError(e) => write!(f, "Receiving channel failed: {}", e),
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

impl From<std::sync::mpsc::RecvError> for Error {
    fn from(error: std::sync::mpsc::RecvError) -> Self {
        Error::ChannelReceiverError(error)
    }
}

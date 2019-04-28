use crate::data::message::MessageType;
use crate::Error;
use bytes::{Bytes, BytesMut};
use futures::sync::mpsc;

/// Messages sent to the server by the connections for example
pub enum ConnectionMessage {
    NewConnection(tokio::net::TcpStream),
    Disconnect(Error, String),
    Register(mpsc::Sender<ServerMessage>, String),
    CheckInv(crate::data::message::Inv, String),
    Retrieve(crate::data::message::GetData, String),
    SyncBlocks(crate::data::message::GetBlocks, String),
    NewTransaction(crate::data::ressources::Transaction),
}

/// Messages Sent From the server
#[derive(Debug)]
pub enum ServerMessage {
    Tick,
    Terminate(Error),
    SendMessage(MessageType, Bytes),
    HandleMessage(MessageType, BytesMut),
}

impl std::fmt::Display for ConnectionMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ConnectionMessage::NewConnection(_) => "NewConnection",
                ConnectionMessage::Disconnect(_, _) => "Disconnect",
                ConnectionMessage::Register(_, _) => "Register",
                ConnectionMessage::CheckInv(_, _) => "CheckInv",
                ConnectionMessage::Retrieve(_, _) => "Retrieve",
                ConnectionMessage::SyncBlocks(_, _) => "SyncBlocks",
                ConnectionMessage::NewTransaction(_) => "NewTx",
            }
        )
    }
}

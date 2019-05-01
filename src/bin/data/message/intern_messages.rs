use crate::data::message::MessageType;
use crate::Error;
use bytes::{Bytes, BytesMut};
use futures::sync::mpsc;

/// Messages sent to the server by the connections for example
pub enum ConnectionMessage {
    Disconnect(Error, String),
    CheckInv(crate::data::message::Inv, String),
    Retrieve(crate::data::message::GetData, String),
    SyncBlocks(crate::data::message::GetBlocks, String),
    NewTransaction(crate::data::ressources::Transaction),
    Connect(std::net::SocketAddr),
    NewPrompt(tokio::net::TcpStream),
    NewConnection(tokio::net::TcpStream),
    Register(mpsc::Sender<ServerMessage>, String),
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
                ConnectionMessage::Disconnect(_, _) => "Disconnect",
                ConnectionMessage::CheckInv(_, _) => "CheckInv",
                ConnectionMessage::Retrieve(_, _) => "Retrieve",
                ConnectionMessage::SyncBlocks(_, _) => "SyncBlocks",
                ConnectionMessage::NewTransaction(_) => "NewTx",
                ConnectionMessage::Connect(_) => "Connect",
                ConnectionMessage::NewConnection(_) => "NewConnection",
                ConnectionMessage::Register(_, _) => "Register",
                ConnectionMessage::NewPrompt(_) => "NewPrompt",
            }
        )
    }
}

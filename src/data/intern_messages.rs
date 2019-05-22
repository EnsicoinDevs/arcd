use crate::Error;
use bytes::{Bytes, BytesMut};
use ensicoin_messages::{
    message::{GetBlocks, GetData, Inv, MessageType},
    resource::{Block, Transaction},
};
use futures::sync::mpsc;

pub enum Source {
    Connection(String),
    RPC,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Source::Connection(r) => format!("connetion [{}]", r),
                Source::RPC => "RPC".to_string(),
            }
        )
    }
}

/// Messages sent to the server by the connections for example
pub enum ConnectionMessage {
    Disconnect(Error, String),
    Clean(String),
    CheckInv(Inv, Source),
    Retrieve(GetData, Source),
    SyncBlocks(GetBlocks, Source),
    NewTransaction(Transaction, Source),
    NewBlock(Block, Source),
    Connect(std::net::SocketAddr),
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

impl Clone for ServerMessage {
    fn clone(&self) -> Self {
        match self {
            ServerMessage::Tick => ServerMessage::Tick,
            ServerMessage::Terminate(_) => ServerMessage::Terminate(Error::ServerTermination),
            ServerMessage::SendMessage(t, a) => ServerMessage::SendMessage(t.clone(), a.clone()),
            ServerMessage::HandleMessage(t, a) => {
                ServerMessage::HandleMessage(t.clone(), a.clone())
            }
        }
    }
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
                ConnectionMessage::NewTransaction(_, _) => "NewTx",
                ConnectionMessage::Connect(_) => "Connect",
                ConnectionMessage::NewConnection(_) => "NewConnection",
                ConnectionMessage::Register(_, _) => "Register",
                ConnectionMessage::NewBlock(_, _) => "NewBlock",
                ConnectionMessage::Clean(_) => "Clean",
            }
        )
    }
}

#[derive(Clone)]
pub enum BroadcastMessage {
    BestBlock(ensicoin_messages::resource::Block),
}

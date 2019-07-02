use crate::Error;
use bytes::{Bytes, BytesMut};
use ensicoin_messages::{
    message::{GetBlocks, GetData, Inv, MessageType},
    resource::{Block, Transaction},
};
use futures::sync::mpsc;

#[derive(Hash, Eq, PartialEq)]
pub enum Source {
    Connection(String),
    Server,
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
                Source::Server => "Server".to_string(),
            }
        )
    }
}

pub struct ConnectionMessage {
    pub content: ConnectionMessageContent,
    pub source: Source,
}

/// Messages sent to the server by the connections for example
pub enum ConnectionMessageContent {
    Disconnect(Error, String),
    Clean(String),
    CheckInv(Inv),
    Retrieve(GetData),
    SyncBlocks(GetBlocks),
    NewTransaction(Transaction),
    NewBlock(Block),
    Connect(std::net::SocketAddr),
    NewConnection(tokio::net::TcpStream),
    Register(mpsc::Sender<ServerMessage>, String),
    Quit,
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
        write!(f, "{} from {}", self.content, self.source)
    }
}

impl std::fmt::Display for ConnectionMessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ConnectionMessageContent::Disconnect(_, _) => "Disconnect",
                ConnectionMessageContent::CheckInv(_) => "CheckInv",
                ConnectionMessageContent::Retrieve(_) => "Retrieve",
                ConnectionMessageContent::SyncBlocks(_) => "SyncBlocks",
                ConnectionMessageContent::NewTransaction(_) => "NewTx",
                ConnectionMessageContent::Connect(_) => "Connect",
                ConnectionMessageContent::NewConnection(_) => "NewConnection",
                ConnectionMessageContent::Register(_, _) => "Register",
                ConnectionMessageContent::NewBlock(_) => "NewBlock",
                ConnectionMessageContent::Clean(_) => "Clean",
                ConnectionMessageContent::Quit => "Quit",
            }
        )
    }
}

#[derive(Clone)]
pub enum BroadcastMessage {
    BestBlock(ensicoin_messages::resource::Block),
}

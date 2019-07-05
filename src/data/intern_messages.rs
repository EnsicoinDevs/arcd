use crate::Error;
use bytes::{Bytes, BytesMut};
use ensicoin_messages::{
    message::{GetBlocks, GetData, Inv, MessageType},
    resource::{Block, Transaction},
};
use ensicoin_serializer::{Deserialize, Deserializer, Serialize};
use futures::sync::mpsc;

#[derive(Eq, PartialEq)]
pub enum Source {
    Connection(RemoteIdentity),
    Server,
    RPC,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Source::Connection(r) => format!("connetion [{}]", r.tcp_address),
                Source::RPC => "RPC".to_string(),
                Source::Server => "Server".to_string(),
            }
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Hash)]
pub struct RemoteIdentity {
    pub tcp_address: String,
    pub peer: Peer,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Hash, Copy)]
pub struct Peer {
    pub ip: [u8; 16],
    pub port: u16,
}

impl Deserialize for Peer {
    fn deserialize(de: &mut Deserializer) -> ensicoin_serializer::Result<Self> {
        let ip_bytes = de.extract_bytes(16)?;
        let mut ip = [0; 16];
        for (i, b) in ip_bytes.iter().enumerate() {
            ip[i] = *b;
        }
        let port = u16::deserialize(de)?;

        Ok(Peer { ip, port })
    }
}

impl Serialize for Peer {
    fn serialize(&self) -> Bytes {
        let mut buf = Bytes::new();
        buf.extend_from_slice(&self.ip);
        buf.extend_from_slice(&self.port.serialize());
        buf
    }
}

pub struct ConnectionMessage {
    pub content: ConnectionMessageContent,
    pub source: Source,
}

/// Messages sent to the server by the connections for example
pub enum ConnectionMessageContent {
    Disconnect(Error, String),
    Clean(RemoteIdentity),
    CheckInv(Inv),
    Retrieve(GetData),
    SyncBlocks(GetBlocks),
    NewTransaction(Transaction),
    NewBlock(Block),
    Connect(std::net::SocketAddr),
    NewConnection(tokio::net::TcpStream),
    Register(mpsc::Sender<ServerMessage>, RemoteIdentity),
    RetrieveAddr,
    ConnectionFailed(std::net::SocketAddr),
    NewAddr(ensicoin_messages::message::Addr),
    VerifiedAddr(ensicoin_messages::message::Address),
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
                ConnectionMessageContent::RetrieveAddr => "RetrieveAddr",
                ConnectionMessageContent::NewAddr(_) => "NewAddr",
                ConnectionMessageContent::VerifiedAddr(_) => "VerifiedAddr",
                ConnectionMessageContent::ConnectionFailed(_) => "ConnectionFailed",
                ConnectionMessageContent::Quit => "Quit",
            }
        )
    }
}

#[derive(Clone)]
pub enum BroadcastMessage {
    BestBlock(ensicoin_messages::resource::Block),
    Quit,
}

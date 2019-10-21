use tokio::{net::TcpStream, prelude::*, sync::mpsc};

use crate::{
    data::{
        intern_messages::{self, ConnectionMessage, ConnectionMessageContent, ServerMessage},
        MessageCodec, MessageCodecError,
    },
    network::create_self_address,
    Error,
};
use ensicoin_messages::message::{Message, MessageType, Whoami};

type ConnectionSender = mpsc::Sender<ConnectionMessage>;

const CHANNEL_CAPACITY: usize = 2_048;

#[derive(Clone, Copy, Debug)]
pub enum TerminationReason {
    RequestedTermination,
    TooManyConnections,
    Quit,
}

#[derive(Debug)]
pub enum CreationError {
    HandshakeError,
    IoError(tokio::io::Error),
    TimedOut,
}
#[derive(Debug)]
pub enum ConnectionError {
    TerminatedByServer {
        reason: TerminationReason,
    },
    IoError(tokio::io::Error),
    InvalidState {
        message_type: MessageType,
        state: State,
    },
    SendToServer,
    RecvFromServer,
    InvalidMessage(MessageCodecError),
}
impl From<tokio::io::Error> for ConnectionError {
    fn from(err: tokio::io::Error) -> Self {
        Self::IoError(err)
    }
}
impl From<mpsc::error::RecvError> for ConnectionError {
    fn from(_: mpsc::error::RecvError) -> Self {
        Self::RecvFromServer
    }
}
impl From<mpsc::error::SendError> for ConnectionError {
    fn from(_: mpsc::error::SendError) -> Self {
        Self::SendToServer
    }
}
impl From<MessageCodecError> for ConnectionError {
    fn from(err: MessageCodecError) -> Self {
        Self::InvalidMessage(err)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum State {
    Initiated,
    Replied,
    Idle,
    Confirm,
    Ack,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:#}", self)
    }
}

type FramedStream = tokio::codec::Framed<tokio::net::TcpStream, MessageCodec>;

pub struct Connection {
    state: State,
    connection_sender: mpsc::Sender<ConnectionMessage>,
    server_sender: mpsc::Sender<ServerMessage>,
    reciever: mpsc::Receiver<ServerMessage>,
    frame: FramedStream,
    version: u32,
    remote: String,
    waiting_ping: bool,
    origin_port: u16,
    identity: crate::data::intern_messages::RemoteIdentity,
}

impl Connection {
    fn new(stream: TcpStream, sender: ConnectionSender, origin_port: u16) -> Connection {
        let (sender_to_connection, reciever) = mpsc::channel(CHANNEL_CAPACITY);
        let remote = stream.peer_addr().unwrap().to_string();
        let frame = tokio::codec::Framed::new(stream, MessageCodec::new());

        let mut identity = crate::data::intern_messages::RemoteIdentity::default();
        identity.tcp_address = remote.clone();
        Connection {
            state: State::Idle,
            frame,
            version: crate::constants::VERSION,
            remote: remote.clone(),
            connection_sender: sender.clone(),
            server_sender: sender_to_connection.clone(),
            reciever,
            waiting_ping: false,
            origin_port,
            identity,
        }
    }
    pub async fn initiate(
        address: std::net::SocketAddr,
        sender: ConnectionSender,
        origin_port: u16,
    ) -> Result<(), CreationError> {
        let stream = match tokio::timer::Timeout::new(
            tokio::net::TcpStream::connect(&address),
            std::time::Duration::from_secs(2),
        )
        .await
        {
            Ok(Ok(s)) => s,
            Err(_) => return Err(CreationError::TimedOut),
            Ok(Err(e)) => return Err(CreationError::IoError(e)),
        };
        let remote = stream.peer_addr().unwrap().to_string();
        info!("connected to [{}]", remote);
        let mut conn = Connection::new(stream, sender, origin_port);
        let msg = Message::Whoami(Whoami::new(create_self_address(origin_port)));
        conn.state = State::Initiated;
        if let Err(e) = conn.frame.send(msg).await {
            warn!("Could not create connection: {:?}", e);
            return Err(CreationError::HandshakeError);
        };
        tokio::spawn(conn.run());
        Ok(())
    }
    pub fn accept(stream: TcpStream, sender: ConnectionSender, origin_port: u16) {
        let connection = Connection::new(stream, sender, origin_port);
        tokio::spawn(connection.run());
    }
    async fn run(self) {
        let timer = tokio::timer::Interval::new_interval(std::time::Duration::from_secs(42));
        // TODO !
        /*match msg {
            ServerMessage::Terminate(e) => {
                info!("Terminate for {:?}", e);
                self.terminate(e)?;
            }
            ServerMessage::SendMsg(m) => self.send(m).await?,
        }*/
    }

    async fn send_message(
        &mut self,
        content: intern_messages::ConnectionMessageContent,
    ) -> Result<(), tokio::sync::mpsc::error::SendError> {
        self.connection_sender
            .send(intern_messages::ConnectionMessage {
                source: self.source(),
                content,
            })
            .await
    }

    pub fn source(&self) -> intern_messages::Source {
        intern_messages::Source::Connection(self.identity.clone())
    }

    pub fn remote(&self) -> &str {
        &self.remote
    }

    pub async fn send(&mut self, msg: Message) -> Result<(), ConnectionError> {
        let t = msg.message_type();
        if self.state == State::Ack || t == MessageType::Whoami || t == MessageType::WhoamiAck {
            match t {
                MessageType::Ping | MessageType::Pong => {
                    debug!("Sending {} to [{}]", t, self.remote())
                }
                _ => info!("Sending {} to [{}]", t, self.remote()),
            };
            self.frame.send(msg).await?;
            Ok(())
        } else {
            Err(ConnectionError::InvalidState {
                state: self.state,
                message_type: t,
            })
        }
    }

    async fn terminate(&mut self, error: Error) {
        warn!("connection [{}] terminated: {}", self.remote(), error);
        if let Err(e) = self
            .connection_sender
            .send(ConnectionMessage {
                content: ConnectionMessageContent::Clean(self.identity.clone()),
                source: self.source(),
            })
            .await
        {
            warn!("Could not terminate gracefully connection: {:?}", e);
        }
    }

    async fn handle_message(&mut self, msg: Message) -> Result<(), ConnectionError> {
        match msg {
            Message::Whoami(remote_id) if self.state == State::Idle => {
                let resp = Message::Whoami(Whoami::new(create_self_address(self.origin_port)));
                self.send(resp).await?;

                let ack = Message::WhoamiAck;
                self.send(ack).await?;

                self.identity.peer.ip = remote_id.address.ip;
                self.identity.peer.port = remote_id.address.port;
                self.version = std::cmp::min(self.version, remote_id.version);
                self.state = State::Confirm;
            }
            Message::Whoami(_) if self.state == State::Initiated => {
                self.state = State::Replied;
            }
            Message::WhoamiAck if self.state == State::Confirm => {
                self.state = State::Ack;
                self.send_message(ConnectionMessageContent::Register(
                    self.server_sender.clone(),
                    self.identity.clone(),
                ))
                .await?;
            }
            Message::WhoamiAck if self.state == State::Replied => {
                self.state = State::Ack;
                self.send_message(ConnectionMessageContent::Register(
                    self.server_sender.clone(),
                    self.identity.clone(),
                ))
                .await?;
                self.send(Message::WhoamiAck).await?;
            }
            Message::Whoami(_) => {
                warn!("[{}] is not in a state accepting whoami", self.remote());
            }
            Message::WhoamiAck => {
                warn!("[{}] is not in a state accepting whoamiack", self.remote());
            }
            Message::Inv(inv) => {
                self.send_message(ConnectionMessageContent::CheckInv(inv))
                    .await?;
            }
            Message::GetData(get_data) => {
                self.send_message(ConnectionMessageContent::Retrieve(get_data))
                    .await?;
            }
            Message::NotFound(_) => (),
            Message::Block(block) => {
                self.send_message(ConnectionMessageContent::NewBlock(block))
                    .await?
            }
            Message::GetBlocks(get_blocks) => {
                self.send_message(ConnectionMessageContent::SyncBlocks(get_blocks))
                    .await?;
            }
            // TODO: getMempool
            Message::GetMempool => (),
            Message::Tx(tx) => {
                self.send_message(ConnectionMessageContent::NewTransaction(tx))
                    .await?
            }

            Message::Ping => {
                self.send(Message::Pong).await?;
            }
            Message::Pong => {
                self.waiting_ping = false;
            }
            Message::GetAddr => {
                self.send_message(ConnectionMessageContent::RetrieveAddr)
                    .await?;
            }
            Message::Addr(addrs) => {
                self.send_message(ConnectionMessageContent::NewAddr(addrs))
                    .await?
            }
        };
        Ok(())
    }
}

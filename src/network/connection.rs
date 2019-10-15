use tokio::{net::TcpStream, prelude::*, sync::mpsc};

use ensicoin_serializer::{Deserialize, Deserializer};

use bytes::{Bytes, BytesMut};

use crate::{
    data::{
        intern_messages::{self, ConnectionMessage, ConnectionMessageContent, ServerMessage},
        MessageCodec,
    },
    network::create_self_address,
    Error,
};
use ensicoin_messages::{
    message::{self, Addr, GetBlocks, GetData, Inv, Message, MessageType, Ping, Whoami, WhoamiAck},
    resource::Transaction,
};

type ConnectionSender = mpsc::Sender<ConnectionMessage>;

const CHANNEL_CAPACITY: usize = 2_048;

#[derive(Debug)]
pub enum CreationError {
    HandshakeError,
    IoError(tokio::io::Error),
    TimedOut,
}

#[derive(PartialEq, Eq, Debug, Clone)]
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
    frame: FramedStream,
    version: u32,
    remote: String,
    waiting_ping: bool,
    origin_port: u16,
    identity: crate::data::intern_messages::RemoteIdentity,
}

impl Connection {
    fn handle_server_message(&mut self, msg: ServerMessage) -> Result<(), ()> {
        match msg {
            ServerMessage::Terminate(e) => {
                self.terminate(e);
                return Err(());
            }
            ServerMessage::SendMessage(t, v) => {
                if let Err(e) = self.buffer_message(t, v) {
                    self.terminate(e);
                    return Err(());
                }
            }
            ServerMessage::HandleMessage(t, v) => {
                match t {
                    ensicoin_messages::message::MessageType::Ping
                    | ensicoin_messages::message::MessageType::Pong => {
                        trace!("{} from [{}]", t, self.remote())
                    }
                    _ => info!("{} from [{}]", t, self.remote()),
                };
                if let Err(e) = self.handle_message(t, v) {
                    self.terminate(e);
                    return Err(());
                }
            }
            ServerMessage::Tick => {
                if self.waiting_ping {
                    self.terminate(Error::NoResponse);
                    return Err(());
                } else {
                    let (t, v) = Ping::new().raw_bytes();
                    if let Err(e) = self.buffer_message(t, v) {
                        self.terminate(e);
                        return Err(());
                    }
                    self.waiting_ping = true;
                }
            }
        }
        Ok(())
    }

    fn create_message(
        &self,
        content: intern_messages::ConnectionMessageContent,
    ) -> intern_messages::ConnectionMessage {
        intern_messages::ConnectionMessage {
            source: self.source(),
            content,
        }
    }

    pub fn source(&self) -> intern_messages::Source {
        intern_messages::Source::Connection(self.identity.clone())
    }
    pub fn new(stream: TcpStream, sender: ConnectionSender, origin_port: u16) -> Connection {
        let (sender_to_connection, reciever) = mpsc::channel(CHANNEL_CAPACITY);
        let remote = stream.peer_addr().unwrap().to_string();
        let (message_sink, message_stream) =
            tokio::codec::Framed::new(stream, MessageCodec::new()).split();

        let timer = tokio::timer::Interval::new_interval(std::time::Duration::from_secs(42))
            .map(|_| ServerMessage::Tick)
            .map_err(Error::TimerError);

        let message_stream = reciever
            .map_err(|_| Error::ChannelError)
            .select(message_stream.map(|raw| {
                let (t, v) = raw;
                ServerMessage::HandleMessage(t, v)
            }))
            .select(timer);

        let mut identity = crate::data::intern_messages::RemoteIdentity::default();
        identity.tcp_address = remote.clone();
        Connection {
            state: State::Idle,
            message_stream: Box::new(message_stream),
            message_sink: Box::new(message_sink),
            message_buffer: std::collections::VecDeque::new(),
            server_buffer: std::collections::VecDeque::new(),
            version: crate::constants::VERSION,
            remote: remote.clone(),
            connection_sender: sender.clone(),
            server_sender: sender_to_connection.clone(),
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
        let (t, v) = Whoami::new(create_self_address(origin_port)).raw_bytes();
        conn.state = State::Initiated;
        if let Err(e) = conn.frame.send(v).await {
            warn!("Could not create connection: {:?}", e);
            return Err(CreationError::HandshakeError);
        };
        Ok(())
    }
    pub async fn run(self) {}

    pub fn remote(&self) -> &str {
        &self.remote
    }

    pub async fn send(&mut self, t: MessageType, v: Bytes) -> Result<(), Error> {
        if self.state == State::Ack || t == MessageType::Whoami || t == MessageType::WhoamiAck {
            match t {
                MessageType::Ping | MessageType::Pong => {
                    debug!("buffering {} for [{}]", t, self.remote())
                }
                _ => info!("buffering {} for [{}]", t, self.remote()),
            };
            self.frame.send(v).await?;
            Ok(())
        } else {
            Err(Error::InvalidConnectionState(format!("{}", self.state)))
        }
    }

    fn terminate(&mut self, error: Error) {
        warn!("connection [{}] terminated: {}", self.remote(), error);
        tokio::spawn(
            self.connection_sender
                .clone()
                .send(ConnectionMessage {
                    content: ConnectionMessageContent::Clean(self.identity.clone()),
                    source: self.source(),
                })
                .map(|_| ())
                .map_err(|e| warn!("Cannot terminate connection gracefully: {}", e)),
        );
    }

    fn handle_message(&mut self, t: MessageType, v: BytesMut) -> Result<(), Error> {
        let mut de = Deserializer::new(v);
        match t {
            MessageType::Whoami if self.state == State::Idle => {
                let (t, v) = Whoami::new(create_self_address(self.origin_port)).raw_bytes();
                self.buffer_message(t, v)?;

                let (t, v) = WhoamiAck::new().raw_bytes();
                self.buffer_message(t, v)?;

                let remote_id = Whoami::deserialize(&mut de)?;
                self.identity.peer.ip = remote_id.address.ip;
                self.identity.peer.port = remote_id.address.port;
                self.version = std::cmp::min(self.version, remote_id.version);
                self.state = State::Confirm;
            }
            MessageType::Whoami if self.state == State::Initiated => {
                self.state = State::Replied;
            }
            MessageType::WhoamiAck if self.state == State::Confirm => {
                self.state = State::Ack;
                self.server_buffer.push_back(self.create_message(
                    ConnectionMessageContent::Register(
                        self.server_sender.clone(),
                        self.identity.clone(),
                    ),
                ));
            }
            MessageType::WhoamiAck if self.state == State::Replied => {
                self.state = State::Ack;
                self.server_buffer.push_back(self.create_message(
                    ConnectionMessageContent::Register(
                        self.server_sender.clone(),
                        self.identity.clone(),
                    ),
                ));
                let (t, v) = WhoamiAck::new().raw_bytes();
                self.buffer_message(t, v)?;
            }
            MessageType::Whoami => {
                warn!("[{}] is not in a state accepting whoami", self.remote());
            }
            MessageType::WhoamiAck => {
                warn!("[{}] is not in a state accepting whoamiack", self.remote());
            }
            MessageType::Inv => {
                self.server_buffer.push_back(self.create_message(
                    ConnectionMessageContent::CheckInv(Inv::deserialize(&mut de)?),
                ));
            }
            MessageType::GetData => {
                self.server_buffer.push_back(self.create_message(
                    ConnectionMessageContent::Retrieve(GetData::deserialize(&mut de)?),
                ));
            }
            MessageType::NotFound => (),
            MessageType::Block => self.server_buffer.push_back(self.create_message(
                ConnectionMessageContent::NewBlock(
                    ensicoin_messages::resource::Block::deserialize(&mut de)?,
                ),
            )),
            MessageType::GetBlocks => {
                self.server_buffer.push_back(self.create_message(
                    ConnectionMessageContent::SyncBlocks(GetBlocks::deserialize(&mut de)?),
                ));
            }
            // TODO: getMempool
            MessageType::GetMempool => (),
            MessageType::Transaction => {
                self.server_buffer.push_back(self.create_message(
                    ConnectionMessageContent::NewTransaction(Transaction::deserialize(&mut de)?),
                ));
            }

            MessageType::Unknown(_) => {
                warn!("unknown message type ({}) from [{}]", t, self.remote());
            }
            MessageType::Ping => {
                let (t, v) = message::Pong::new().raw_bytes();
                self.buffer_message(t, v)?;
            }
            MessageType::Pong => {
                self.waiting_ping = false;
            }
            MessageType::GetAddr => {
                self.server_buffer
                    .push_back(self.create_message(ConnectionMessageContent::RetrieveAddr));
            }
            MessageType::Addr => self.server_buffer.push_back(self.create_message(
                ConnectionMessageContent::NewAddr(Addr::deserialize(&mut de)?),
            )),
        };
        Ok(())
    }
}

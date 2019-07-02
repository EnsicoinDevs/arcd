use futures::sync::mpsc;
use tokio::{net::TcpStream, prelude::*};

use ensicoin_serializer::{Deserialize, Deserializer};

use bytes::{Bytes, BytesMut};

use crate::{
    data::{
        intern_messages::{self, ConnectionMessage, ConnectionMessageContent, ServerMessage},
        MessageCodec,
    },
    Error,
};
use ensicoin_messages::{
    message::{self, GetBlocks, GetData, Inv, Message, MessageType, Ping, Whoami, WhoamiAck},
    resource::Transaction,
};

type ConnectionSender = mpsc::Sender<ConnectionMessage>;

const CHANNEL_CAPACITY: usize = 2_048;

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

pub struct Connection {
    state: State,
    connection_sender: mpsc::Sender<ConnectionMessage>,
    server_sender: mpsc::Sender<ServerMessage>,
    message_stream: Box<dyn futures::Stream<Item = ServerMessage, Error = Error> + Send>,
    message_sink: Box<dyn Sink<SinkItem = Bytes, SinkError = Error> + Send>,
    message_buffer: std::collections::VecDeque<(MessageType, Bytes)>,
    server_buffer: std::collections::VecDeque<ConnectionMessage>,
    version: u32,
    remote: String,
    waiting_ping: bool,
    termination: bool,
    terminator: Terminator,
}

impl Connection {
    fn handle_server_message(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::Terminate(e) => {
                self.terminate(e);
            }
            ServerMessage::SendMessage(t, v) => {
                if let Err(e) = self.buffer_message(t, v) {
                    self.terminate(e);
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
                }
            }
            ServerMessage::Tick => {
                if self.waiting_ping {
                    self.terminate(Error::NoResponse);
                } else {
                    let (t, v) = Ping::new().raw_bytes();
                    if let Err(e) = self.buffer_message(t, v) {
                        self.terminate(e);
                    }
                    self.waiting_ping = true;
                }
            }
        }
    }
}

impl futures::Future for Connection {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if !self.termination {
            loop {
                trace!("Polled");
                match self.message_sink.poll_complete() {
                    Ok(Async::Ready(_)) => {
                        trace!("Sent message to remote");
                    }
                    Ok(Async::NotReady) => {
                        trace!("Sink has not sent");
                        return Ok(Async::NotReady);
                    }
                    Err(e) => {
                        self.terminate(e);
                    }
                }
                debug!("Message count: {}", self.message_buffer.len());
                while !self.message_buffer.is_empty() {
                    let (t, v) = self.message_buffer.pop_front().unwrap();
                    match t {
                        ensicoin_messages::message::MessageType::Ping
                        | ensicoin_messages::message::MessageType::Pong => {
                            trace!("Sending {} to [{}]", t, self.remote())
                        }
                        _ => info!("Sending {} to [{}]", t, self.remote()),
                    };
                    match self.message_sink.start_send(v) {
                        Ok(AsyncSink::Ready) => {
                            debug!("Started sending");
                        }
                        Ok(AsyncSink::NotReady(msg)) => {
                            self.message_buffer.push_front((t, msg));
                            trace!("Sender not ready, queued");
                            return Ok(Async::NotReady);
                        }
                        Err(e) => {
                            self.terminate(e);
                        }
                    }
                }
                debug!("Finished sending messages");
                match self.message_sink.poll_complete() {
                    Ok(Async::Ready(_)) => {
                        trace!("Sent message to remote");
                    }
                    Ok(Async::NotReady) => {
                        trace!("Sink has not sent");
                        return Ok(Async::NotReady);
                    }
                    Err(e) => {
                        self.terminate(e);
                    }
                }
                match self.connection_sender.poll_complete() {
                    Ok(Async::Ready(_)) => {
                        trace!("Sent message to server");
                    }
                    Ok(Async::NotReady) => {
                        trace!("Can't send message, not ready");
                        return Ok(Async::NotReady);
                    }
                    Err(e) => panic!("Connection can't communicate with server: {}", e),
                }
                while !self.server_buffer.is_empty() {
                    match self
                        .connection_sender
                        .start_send(self.server_buffer.pop_front().unwrap())
                    {
                        Ok(AsyncSink::Ready) => (),
                        Ok(AsyncSink::NotReady(msg)) => {
                            self.server_buffer.push_front(msg);
                            trace!("Server message sink not ready");
                        }
                        Err(e) => panic!("Connection can't communicate with server: {}", e),
                    }
                }
                debug!("Finished sending server messages");
                match self.connection_sender.poll_complete() {
                    Ok(Async::Ready(_)) => {
                        trace!("Sent message to server");
                    }
                    Ok(Async::NotReady) => {
                        trace!("Can't send message, not ready");
                        return Ok(Async::NotReady);
                    }
                    Err(e) => panic!("Connection can't communicate with server: {}", e),
                }

                match self.message_stream.poll() {
                    Ok(Async::Ready(None)) => (),
                    Ok(Async::Ready(Some(msg))) => {
                        trace!("Handling server message: {:?}", msg);
                        self.handle_server_message(msg);
                    }
                    Ok(Async::NotReady) => {
                        debug!("Waiting connection event");
                        return Ok(Async::NotReady);
                    }
                    Err(e) => {
                        self.terminate(e);
                        return Err(());
                    }
                }
            }
        } else {
            self.terminator.poll()
        }
    }
}

impl futures::Future for Terminator {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.sender.start_send(ConnectionMessage {
            content: ConnectionMessageContent::Clean(self.remote.clone()),
            source: intern_messages::Source::Connection(self.remote.clone()),
        }) {
            Ok(AsyncSink::NotReady(_)) => return Ok(Async::NotReady),
            Ok(AsyncSink::Ready) => self.staged = true,
            Err(e) => panic!("Can't terminate: {}", e),
        };
        self.sender
            .poll_complete()
            .map_err(|e| panic!("Can't terminate: {}", e))
    }
}

struct Terminator {
    sender: mpsc::Sender<ConnectionMessage>,
    error: Option<Error>,
    remote: String,
    staged: bool,
}

impl Terminator {
    fn new(sender: mpsc::Sender<ConnectionMessage>, remote: String) -> Terminator {
        Terminator {
            sender,
            staged: false,
            error: None,
            remote,
        }
    }

    fn set_error(&mut self, error: Error) {
        self.error = Some(error);
    }
}

impl Connection {
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
        intern_messages::Source::Connection(self.remote.clone())
    }
    pub fn new(stream: TcpStream, sender: ConnectionSender) -> Connection {
        let (sender_to_connection, reciever) = mpsc::channel(CHANNEL_CAPACITY);
        let remote = stream.peer_addr().unwrap().to_string();
        let (message_sink, message_stream) =
            tokio::codec::Framed::new(stream, MessageCodec::new()).split();

        let timer = tokio_timer::Interval::new_interval(std::time::Duration::from_secs(42))
            .map(|_| ServerMessage::Tick)
            .map_err(Error::TimerError);

        let message_stream = reciever
            .map_err(|_| Error::ChannelError)
            .select(message_stream.map(|raw| {
                let (t, v) = raw;
                ServerMessage::HandleMessage(t, v)
            }))
            .select(timer);

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
            termination: false,
            terminator: Terminator::new(sender, remote),
        }
    }

    pub fn initiate(address: &std::net::SocketAddr, sender: ConnectionSender) {
        tokio::spawn(
            tokio::net::TcpStream::connect(address)
                .map_err(|_| ())
                .and_then(|stream| {
                    let remote = stream.peer_addr().unwrap().to_string();
                    info!("connected to [{}]", remote);
                    let mut conn = Connection::new(stream, sender);
                    let (t, v) = Whoami::new().raw_bytes();
                    conn.state = State::Initiated;
                    conn.buffer_message(t, v).unwrap();
                    conn
                }),
        );
    }

    pub fn remote(&self) -> &str {
        &self.remote
    }

    pub fn buffer_message(&mut self, t: MessageType, v: Bytes) -> Result<(), Error> {
        if self.state == State::Ack || t == MessageType::Whoami || t == MessageType::WhoamiAck {
            trace!("buffering {} for [{}]", t, self.remote());
            self.message_buffer.push_back((t, v));
            Ok(())
        } else {
            Err(Error::InvalidConnectionState(format!("{}", self.state)))
        }
    }

    fn terminate(&mut self, error: Error) {
        warn!("connection [{}] terminated: {}", self.remote(), error);
        self.termination = true;
        self.terminator.set_error(error);
    }

    fn handle_message(&mut self, t: MessageType, v: BytesMut) -> Result<(), Error> {
        let mut de = Deserializer::new(v);
        match t {
            MessageType::Whoami if self.state == State::Idle => {
                let (t, v) = Whoami::new().raw_bytes();
                self.buffer_message(t, v)?;

                let (t, v) = WhoamiAck::new().raw_bytes();
                self.buffer_message(t, v)?;

                let remote_id = Whoami::deserialize(&mut de)?;
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
                        String::from(self.remote()),
                    ),
                ));
            }
            MessageType::WhoamiAck if self.state == State::Replied => {
                self.state = State::Ack;
                self.server_buffer.push_back(self.create_message(
                    ConnectionMessageContent::Register(
                        self.server_sender.clone(),
                        String::from(self.remote()),
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
            // TODO: Implement Addr
            MessageType::GetAddr => (),
            MessageType::Addr => (),
        };
        Ok(())
    }
}

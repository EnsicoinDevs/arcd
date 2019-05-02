use futures::sync::mpsc;
use tokio::net::TcpStream;
use tokio::prelude::*;

use ensicoin_serializer::{Deserialize, Deserializer};

use bytes::{Bytes, BytesMut};

use crate::data::message;
use crate::data::message::MessageCodec;
use crate::data::message::{ConnectionMessage, ServerMessage};
use crate::data::message::{Message, MessageType, Ping, Whoami, WhoamiAck};
use crate::Error;

type ConnectionSender = mpsc::Sender<ConnectionMessage>;

const CHANNEL_CAPACITY: usize = 1_024;

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

fn channel_stream_error_converter(_: ()) -> Error {
    Error::ChannelError
}
fn raw_message_converter(raw_message: RawMessage) -> ServerMessage {
    let (t, v) = raw_message;
    ServerMessage::HandleMessage(t, v)
}
fn ticker_converter(_: std::time::Instant) -> ServerMessage {
    ServerMessage::Tick
}
fn timer_error_converter(e: tokio_timer::Error) -> Error {
    Error::TimerError(e)
}

type ChannelStream = futures::sync::mpsc::Receiver<ServerMessage>;
type ChannelStreamErrorConverter = fn(()) -> Error;
type ChannelStreamErrored = stream::MapErr<ChannelStream, ChannelStreamErrorConverter>;
type MessageFramedTcpStream = tokio::codec::Framed<TcpStream, MessageCodec>;
type RawMessage = (message::MessageType, BytesMut);
type RawMessageConverter = fn(RawMessage) -> ServerMessage;
type RawMessageStream = stream::Map<MessageStream, RawMessageConverter>;
type PartStream = stream::Select<ChannelStreamErrored, RawMessageStream>;
type TimerStream = tokio_timer::Interval;
type TickerConverter = fn(std::time::Instant) -> ServerMessage;
type TickerStream = stream::Map<TimerStream, TickerConverter>;
type TimerErrorConverter = fn(tokio_timer::Error) -> Error;
type TickerStreamErrored = stream::MapErr<TickerStream, TimerErrorConverter>;
type ConnectionStream = stream::Select<PartStream, TickerStreamErrored>;

type MessageStream = futures::stream::SplitStream<MessageFramedTcpStream>;
type MessageSink = futures::stream::SplitSink<MessageFramedTcpStream>;

pub struct Connection {
    state: State,
    connection_sender: mpsc::Sender<ConnectionMessage>,
    server_sender: mpsc::Sender<ServerMessage>,
    message_stream: ConnectionStream,
    message_sink: MessageSink,
    message_buffer: std::collections::VecDeque<(MessageType, Bytes)>,
    server_buffer: std::collections::VecDeque<ConnectionMessage>,
    version: u32,
    remote: String,
    waiting_ping: bool,
    termination: bool,
    terminator: Terminator,
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
                    info!("Sending {} to [{}]", t, self.remote());
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
                                info!("{} from [{}]", t, self.remote());
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
            return self.terminator.poll();
        }
    }
}

impl futures::Future for Terminator {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.sender.start_send(ConnectionMessage::Disconnect(
            self.error.take().unwrap(),
            String::from(self.remote.clone()),
        )) {
            Ok(AsyncSink::NotReady(_)) => return Ok(Async::NotReady),
            Ok(AsyncSink::Ready) => self.staged = true,
            Err(e) => panic!("Can't terminate: {}", e),
        };
        return self
            .sender
            .poll_complete()
            .map_err(|e| panic!("Can't terminate: {}", e));
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
    pub fn new(stream: TcpStream, sender: ConnectionSender) -> Connection {
        let (sender_to_connection, reciever) = mpsc::channel(CHANNEL_CAPACITY);
        let remote = stream.peer_addr().unwrap().to_string();
        let (message_sink, message_stream) =
            tokio::codec::Framed::new(stream, crate::data::message::MessageCodec::new()).split();

        let timer = tokio_timer::Interval::new_interval(std::time::Duration::from_secs(42))
            .map(ticker_converter as TickerConverter)
            .map_err(timer_error_converter as TimerErrorConverter);

        let message_stream = reciever
            .map_err(channel_stream_error_converter as ChannelStreamErrorConverter)
            .select(message_stream.map(raw_message_converter as RawMessageConverter))
            .select(timer);

        Connection {
            state: State::Idle,
            message_stream: message_stream,
            message_sink: message_sink,
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
                self.server_buffer.push_back(ConnectionMessage::Register(
                    self.server_sender.clone(),
                    String::from(self.remote()),
                ));
            }
            MessageType::WhoamiAck if self.state == State::Replied => {
                self.state = State::Ack;
                self.server_buffer.push_back(ConnectionMessage::Register(
                    self.server_sender.clone(),
                    String::from(self.remote()),
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
                self.server_buffer.push_back(ConnectionMessage::CheckInv(
                    crate::data::message::Inv::deserialize(&mut de)?,
                    self.remote().to_string(),
                ));
            }
            MessageType::GetData => {
                self.server_buffer.push_back(ConnectionMessage::Retrieve(
                    crate::data::message::GetData::deserialize(&mut de)?,
                    self.remote().to_string(),
                ));
            }
            MessageType::NotFound => (),
            MessageType::Block => (),
            MessageType::GetBlocks => {}
            MessageType::Transaction => {
                self.server_buffer
                    .push_back(ConnectionMessage::NewTransaction(
                        crate::data::ressources::Transaction::deserialize(&mut de)?,
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
        };
        Ok(())
    }
}

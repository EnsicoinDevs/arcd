use std::io::Read;
use std::io::Write;

extern crate ensicoin_serializer;
use ensicoin_serializer::{Deserialize, Deserializer};

use crate::constants::MAGIC;
use crate::data::{Message, MessageType, Whoami, WhoamiAck};
use crate::network::ServerMessage;

type ConnectionSender = std::sync::mpsc::Sender<ConnectionMessage>;

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
        write!(
            f,
            "{:#}",
            self /*match self {
                     State::Ack => "Ack",
                     State::Confirm => "Confirm",
                     State::Idle => "Idle",
                     State::Initiated => "Initiated",
                     State::Replied => "Replied",
                 }*/
        )
    }
}

//#[derive(PartialEq, Eq)]
pub enum ConnectionMessage {
    Disconnect(Error, String),
    Register(std::sync::mpsc::Sender<ServerMessage>, String),
}

#[derive(Debug)]
pub enum Error {
    ParseError(ensicoin_serializer::Error),
    InvalidState(State),
    InvalidMagic(u32),
    IoError(std::io::Error),
    ChannelReceiverError(std::sync::mpsc::RecvError),
    ChannelError,
    ServerTermination,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
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

pub struct Connection {
    state: State,
    stream: std::net::TcpStream,
    connection_sender: std::sync::mpsc::Sender<ConnectionMessage>,
    server_receiver: std::sync::mpsc::Receiver<ServerMessage>,
    server_sender: std::sync::mpsc::Sender<ServerMessage>,
    version: u32,
    remote: String,
}

fn read_message(stream: &mut std::net::TcpStream) -> Result<(MessageType, Vec<u8>), Error> {
    let mut buffer: [u8; 24] = [0; 24];
    stream.read_exact(&mut buffer)?;
    let mut de = Deserializer::new(buffer.to_vec());

    let magic = u32::deserialize(&mut de).unwrap_or(0);
    if magic != MAGIC {
        return Err(Error::InvalidMagic(magic));
    };
    let message_type = de
        .extract_bytes(12)
        .unwrap_or(vec![117, 110, 107, 110, 111, 119, 110]); // "unknown"
    let payload_length = u64::deserialize(&mut de).unwrap_or(0) as usize;

    let message_type = String::from_utf8(message_type).unwrap();
    let message_type = match message_type.as_ref() {
        "whoami\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}" => MessageType::Whoami,
        "whoamiack\u{0}\u{0}\u{0}" => MessageType::WhoamiAck,
        _ => MessageType::Unknown(message_type),
    };

    info!(
        "{:?} from [{}]",
        message_type,
        stream.peer_addr().unwrap().to_string()
    );

    let mut payload: Vec<u8> = vec![0; payload_length];
    stream.read_exact(&mut payload)?;
    trace!(
        "{} read out of {} in [{}]",
        payload.len(),
        payload_length,
        stream.peer_addr().unwrap().to_string()
    );
    Ok((message_type, payload))
}

fn listen_stream(mut stream: std::net::TcpStream, sender: std::sync::mpsc::Sender<ServerMessage>) {
    std::thread::spawn(move || {
        trace!(
            "Listening for [{}]",
            stream.peer_addr().unwrap().to_string()
        );
        loop {
            match read_message(&mut stream) {
                Ok((message_type, v)) => {
                    sender
                        .send(ServerMessage::HandleMessage(message_type, v))
                        .unwrap();
                }
                Err(read_error) => {
                    sender.send(ServerMessage::Terminate(read_error)).unwrap();
                    break;
                }
            }
        }
    });
}

impl Connection {
    pub fn new(stream: std::net::TcpStream, sender: ConnectionSender) -> Connection {
        let (sender_to_connection, reciever) = std::sync::mpsc::channel();
        let mut conn = Connection {
            state: State::Idle,
            stream: stream.try_clone().unwrap(),
            version: 1,
            remote: "".to_string(),
            connection_sender: sender,
            server_receiver: reciever,
            server_sender: sender_to_connection.clone(),
        };
        conn.remote = conn.stream.peer_addr().unwrap().to_string();
        listen_stream(stream, sender_to_connection);
        conn
    }

    pub fn idle(mut self) {
        loop {
            let message = self.server_receiver.recv();
            match message {
                Ok(msg) => match msg {
                    ServerMessage::Terminate(e) => {
                        self.terminate(e);
                        break;
                    }
                    ServerMessage::SendMessage(t, v) => {
                        if let Err(e) = self.send_bytes(t, v) {
                            self.terminate(e);
                            break;
                        }
                    }
                    ServerMessage::HandleMessage(t, v) => {
                        if let Err(e) = self.handle_message(t, v) {
                            self.terminate(e);
                            break;
                        }
                    }
                },
                Err(e) => {
                    self.terminate(Error::from(e));
                    break;
                }
            }
        }
    }

    pub fn initiate(
        address: std::net::IpAddr,
        port: u16,
        sender: ConnectionSender,
    ) -> Result<(), Error> {
        let stream = match std::net::TcpStream::connect(std::net::SocketAddr::new(address, port)) {
            Ok(stream) => stream,
            Err(e) => return Err(Error::IoError(e)),
        };
        std::thread::spawn(move || {
            let (sender_to_connection, receiver) = std::sync::mpsc::channel();
            let mut conn = Connection {
                state: State::Initiated,
                stream: stream.try_clone().unwrap(),
                version: 1,
                remote: "".to_string(),
                connection_sender: sender,
                server_receiver: receiver,
                server_sender: sender_to_connection.clone(),
            };

            conn.remote = conn.stream.peer_addr().unwrap().to_string();
            info!("connected to [{}]", conn.remote());

            listen_stream(stream, sender_to_connection);
            match Whoami::new().raw_bytes() {
                Ok((t, v)) => match conn.send_bytes(t, v) {
                    Err(e) => conn.terminate(e),
                    Ok(()) => conn.idle(),
                },
                Err(e) => conn.terminate(e),
            };
        });
        Ok(())
    }

    pub fn remote(&self) -> &str {
        &self.remote
    }

    pub fn send_bytes(&mut self, t: MessageType, v: Vec<u8>) -> Result<(), Error> {
        if self.state == State::Ack || t == MessageType::Whoami || t == MessageType::WhoamiAck {
            if let Err(e) = self.stream.write(&v) {
                return Err(Error::IoError(e));
            };
            info!("{:?} to [{}]", t, self.remote());
            Ok(())
        } else {
            Err(Error::InvalidState(self.state.clone()))
        }
    }

    fn terminate(&mut self, error: Error) {
        warn!("connection [{}] terminated: {}", self.remote(), error);
        self.connection_sender
            .send(ConnectionMessage::Disconnect(
                error,
                String::from(self.remote()),
            ))
            .unwrap();
    }

    fn handle_message(&mut self, t: MessageType, v: Vec<u8>) -> Result<(), Error> {
        let mut de = Deserializer::new(v);
        match t {
            MessageType::Whoami if self.state == State::Idle => {
                let (t, v) = Whoami::new().raw_bytes()?;
                self.send_bytes(t, v)?;

                let (t, v) = WhoamiAck::new().raw_bytes()?;
                self.send_bytes(t, v)?;

                let remote_id = Whoami::deserialize(&mut de)?;
                self.version = std::cmp::min(self.version, remote_id.version);
                self.state = State::Confirm;
            }
            MessageType::Whoami if self.state == State::Initiated => {
                self.state = State::Replied;
            }
            MessageType::WhoamiAck if self.state == State::Confirm => {
                self.state = State::Ack;
                if let Err(_) = self.connection_sender.send(ConnectionMessage::Register(
                    self.server_sender.clone(),
                    String::from(self.remote()),
                )) {
                    return Err(Error::ChannelError);
                };
            }
            MessageType::WhoamiAck if self.state == State::Replied => {
                self.state = State::Ack;
                if let Err(_) = self.connection_sender.send(ConnectionMessage::Register(
                    self.server_sender.clone(),
                    String::from(self.remote()),
                )) {
                    return Err(Error::ChannelError);
                };
                let (t, v) = WhoamiAck::new().raw_bytes()?;
                self.send_bytes(t, v)?;
            }
            MessageType::Whoami => {
                warn!("[{}] is not in a state accepting whoami", self.remote());
            }
            MessageType::WhoamiAck => {
                warn!("[{}] is not in a state accepting whoamiack", self.remote());
            }
            MessageType::Unknown(s) => {
                warn!("unknown message type ({}) from [{}]", s, self.remote());
            }
        };
        Ok(())
    }
}

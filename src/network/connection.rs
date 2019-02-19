use std::io::Read;
use std::io::Write;

extern crate ensicoin_serializer;
use ensicoin_serializer::{Deserialize, Deserializer};

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
            "{}",
            match self {
                State::Ack => "Ack",
                State::Confirm => "Confirm",
                State::Idle => "Idle",
                State::Initiated => "Initiated",
                State::Replied => "Replied",
            }
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
    InvalidState(State),
    IoError(std::io::Error),
    ChannelError,
    ServerTermination,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InvalidState(st) => write!(f, "Connection is in invalid state: {}", st),
            Error::IoError(e) => write!(f, "IoError: {}", e),
            Error::ChannelError => write!(f, "Server channel failed"),
            Error::ServerTermination => write!(f, "Server terminated the connection"),
        }
    }
}

pub struct Connection {
    state: State,
    stream: std::net::TcpStream,
    connection_sender: std::sync::mpsc::Sender<ConnectionMessage>,
    server_reciever: std::sync::mpsc::Receiver<ServerMessage>,
    version: u32,
    remote: String,
}

impl Connection {
    pub fn new(stream: std::net::TcpStream, sender: ConnectionSender) -> Connection {
        let (_, dummy) = std::sync::mpsc::channel();
        let mut conn = Connection {
            state: State::Idle,
            stream,
            version: 1,
            remote: "".to_string(),
            connection_sender: sender,
            server_reciever: dummy,
        };
        conn.remote = conn.stream.peer_addr().unwrap().to_string();
        conn
    }

    pub fn idle(mut self) {
        loop {
            if let Ok(msg) = self.server_reciever.try_recv() {
                match msg {
                    ServerMessage::Terminate => {
                        self.terminate(Error::ServerTermination);
                        break;
                    }
                }
            };
            match self.read_message() {
                Ok((message_type, v)) => match self.handle_message(message_type, v) {
                    Ok(()) => (),
                    Err(parse_error) => {
                        self.terminate(parse_error);
                        break;
                    }
                },
                Err(read_error) => {
                    self.terminate(read_error);
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
            let (_, dummy) = std::sync::mpsc::channel();
            let mut conn = Connection {
                state: State::Initiated,
                stream: stream,
                version: 1,
                remote: "".to_string(),
                connection_sender: sender,
                server_reciever: dummy,
            };

            conn.remote = conn.stream.peer_addr().unwrap().to_string();
            info!("connected to [{}]", conn.remote());

            match Whoami::new().send(&mut conn) {
                Err(e) => conn.terminate(e),
                Ok(()) => conn.idle(),
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

    fn read_message(&mut self) -> Result<(MessageType, Vec<u8>), Error> {
        let mut buffer: [u8; 24] = [0; 24];
        if let Err(e) = self.stream.read_exact(&mut buffer) {
            return Err(Error::IoError(e));
        };
        let mut de = Deserializer::new(buffer.to_vec());

        let magic = u32::deserialize(&mut de).unwrap_or(0);
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

        info!("{:?} from [{}]", message_type, self.remote());

        let mut payload: Vec<u8> = vec![0; payload_length];
        if let Err(e) = self.stream.read_exact(&mut payload) {
            return Err(Error::IoError(e));
        };
        trace!(
            "{} read out of {} in [{}]",
            payload.len(),
            payload_length,
            self.remote()
        );
        Ok((message_type, payload))
    }

    pub fn terminate(&mut self, error: Error) {
        warn!("connection [{}] terminated: {}", self.remote(), error);
        self.connection_sender
            .send(ConnectionMessage::Disconnect(
                error,
                String::from(self.remote()),
            ))
            .unwrap();
    }

    pub fn handle_message(&mut self, t: MessageType, v: Vec<u8>) -> Result<(), Error> {
        match t {
            MessageType::Whoami if self.state == State::Idle => {
                Whoami::new().send(self)?;
                WhoamiAck::new().send(self)?;
                self.state = State::Confirm;
            }
            MessageType::Whoami if self.state == State::Initiated => {
                self.state = State::Replied;
            }
            MessageType::WhoamiAck if self.state == State::Confirm => {
                self.state = State::Ack;
            }
            MessageType::WhoamiAck if self.state == State::Replied => {
                self.state = State::Ack;
                let (sender, reciever) = std::sync::mpsc::channel();
                self.server_reciever = reciever;
                if let Err(_) = self.connection_sender.send(ConnectionMessage::Register(
                    sender,
                    String::from(self.remote()),
                )) {
                    return Err(Error::ChannelError);
                };
                WhoamiAck::new().send(self)?;
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

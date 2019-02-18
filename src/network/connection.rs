use std::io::Read;
use std::io::{ErrorKind, Write};

extern crate ensicoin_serializer;
use ensicoin_serializer::{Deserialize, Deserializer};

use crate::data::{Message, MessageType, Whoami, WhoamiAck};

type ConnectionSender = std::sync::mpsc::Sender<ConnectionMessage>;

#[derive(PartialEq, Eq)]
enum State {
    Initiated,
    Replied,
    Idle,
    Confirm,
    Ack,
}

#[derive(PartialEq, Eq)]
pub enum ConnectionMessage {
    PeerDisconected,
}

pub struct Connection {
    state: State,
    stream: std::net::TcpStream,
    connection_sender: std::sync::mpsc::Sender<ConnectionMessage>,
    version: u32,
    remote: String,
}

impl Connection {
    pub fn new(stream: std::net::TcpStream, sender: ConnectionSender) -> Connection {
        let mut conn = Connection {
            state: State::Idle,
            stream,
            version: 1,
            remote: "".to_string(),
            connection_sender: sender,
        };
        conn.remote = conn.stream.peer_addr().unwrap().to_string();
        conn
    }
    pub fn initiate(
        address: std::net::IpAddr,
        port: u16,
        sender: ConnectionSender,
    ) -> std::io::Result<()> {
        let stream = std::net::TcpStream::connect(std::net::SocketAddr::new(address, port))?;
        std::thread::spawn(move || {
            let mut conn = Connection {
                state: State::Initiated,
                stream: stream,
                version: 1,
                remote: "".to_string(),
                connection_sender: sender,
            };
            conn.remote = conn.stream.peer_addr().unwrap().to_string();
            info!("connected to [{}]", conn.remote());
            Whoami::new().send(&mut conn).unwrap();
            loop {
                match conn.read_message() {
                    Ok((message_type, v)) => match conn.handle_message(message_type, v) {
                        Ok(()) => (),
                        _ => {
                            conn.terminate();
                            break;
                        }
                    },
                    _ => {
                        conn.terminate();
                        break;
                    }
                }
            }
        });
        Ok(())
    }

    pub fn remote(&self) -> &str {
        &self.remote
    }

    pub fn send_bytes(&mut self, t: MessageType, v: Vec<u8>) -> std::io::Result<()> {
        if self.state == State::Ack || t == MessageType::Whoami || t == MessageType::WhoamiAck {
            self.stream.write(&v)?;
            info!("{:?} to [{}]", t, self.remote());
            Ok(())
        } else {
            Err(std::io::Error::from(std::io::ErrorKind::AddrNotAvailable))
        }
    }

    pub fn read_message(&mut self) -> std::io::Result<(MessageType, Vec<u8>)> {
        let mut buffer: [u8; 24] = [0; 24];
        self.stream.read_exact(&mut buffer)?;
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
        self.stream.read_exact(&mut payload)?;
        trace!(
            "{} read out of {} in [{}]",
            payload.len(),
            payload_length,
            self.remote()
        );
        Ok((message_type, payload))
    }

    pub fn terminate(&mut self) {
        self.connection_sender
            .send(ConnectionMessage::PeerDisconected)
            .unwrap();
        warn!("connection disconnected [{}]", self.remote());
    }

    pub fn handle_message(&mut self, t: MessageType, v: Vec<u8>) -> std::io::Result<()> {
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

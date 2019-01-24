use std::io::Read;
use std::io::Write;

extern crate ensicoin_serializer;
use ensicoin_serializer::Deserialize;
use ensicoin_serializer::Deserializer;
use ensicoin_serializer::VarUint;

use crate::data::{Message, MessageType, Whoami, WhoamiAck};

#[derive(PartialEq, Eq)]
enum State {
    Initiated,
    Replied,
    Idle,
    Confirm,
    Ack,
}

pub struct Connection {
    state: State,
    stream: std::net::TcpStream,
    version: u32,
}

impl Connection {
    pub fn new(stream: std::net::TcpStream) -> Connection {
        Connection {
            state: State::Idle,
            stream,
            version: 1,
        }
    }
    pub fn initiate(address: std::net::IpAddr, port: u16) -> std::io::Result<()> {
        let stream = std::net::TcpStream::connect(std::net::SocketAddr::new(address, port))?;
        std::thread::spawn(move || {
            let mut conn = Connection {
                state: State::Initiated,
                stream: stream,
                version: 1,
            };
            info!("connected to [{}]", conn.remote());
            Whoami::new().send(&mut conn).unwrap();
            conn.read_message().unwrap();
        });
        Ok(())
    }

    pub fn remote(&self) -> String {
        self.stream.peer_addr().unwrap().to_string()
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

    pub fn read_message(&mut self) -> std::io::Result<()> {
        let mut buffer: [u8; 24] = [0; 24];
        self.stream.read_exact(&mut buffer)?;
        let mut de = Deserializer::new(buffer.to_vec());

        let magic = u32::deserialize(&mut de).unwrap_or(0);
        let message_type = de
            .extract_bytes(12)
            .unwrap_or(vec![117, 110, 107, 110, 111, 119, 110]); // "unknown"
        let payload_length = VarUint::deserialize(&mut de)
            .unwrap_or(VarUint { value: 0 })
            .value as usize;

        let message_type = String::from_utf8(message_type).unwrap();
        let message_type = match message_type.as_ref() {
            "whoami\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}" => MessageType::Whoami,
            "whoamiack\u{0}\u{0}\u{0}" => MessageType::WhoamiAck,
            _ => MessageType::Unknown(message_type),
        };

        info!("{:?} from [{}]", message_type, self.remote());

        let mut payload: Vec<u8> = vec![0; payload_length];
        self.stream.read_exact(&mut payload)?;
        self.handle_message(message_type, payload);
        Ok(())
    }

    fn handle_message(&mut self, t: MessageType, v: Vec<u8>) {
        match t {
            MessageType::Whoami if self.state == State::Idle => {
                Whoami::new().send(self).unwrap();
                WhoamiAck::new().send(self).unwrap();
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
                WhoamiAck::new().send(self).unwrap();
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
        self.read_message().unwrap();
    }
}

extern crate ensicoin_serializer;
use ensicoin_serializer::Deserialize;
use ensicoin_serializer::Deserializer;
use ensicoin_serializer::VarUint;

use std::io::Read;

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
}

impl Connection {
    pub fn new(stream: std::net::TcpStream) -> Connection {
        Connection {
            state: State::Idle,
            stream,
        }
    }

    pub fn read_header(&mut self) -> std::io::Result<()> {
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
        println!("Raw message type : {:?}", message_type);
        Ok(())
    }
}

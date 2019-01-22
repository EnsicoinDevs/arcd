extern crate ensicoin_serializer;
use ensicoin_serializer::Serialize;

use std::io::Write;
use std::net::TcpStream;

use crate::network::Address;

pub trait Message: Serialize {
    fn message_type() -> [u8; 12];
    fn send(&self, stream: &mut TcpStream) -> Result<(), std::io::Error> {
        let magic: u32 = 422021;
        let message_type = Self::message_type();
        let payload = self.serialize();
        let payload_length: u64 = payload.len() as u64;

        stream.write(&magic.serialize())?;
        stream.write(&message_type)?;
        stream.write(&payload_length.serialize())?;
        stream.write(&payload)?;
        Ok(())
    }
}

pub struct Whoami {
    version: u32,
    address: Address,
    services: Vec<String>,
}

impl Message for Whoami {
    fn message_type() -> [u8; 12] {
        [119, 104, 111, 97, 109, 105, 0, 0, 0, 0, 0, 0]
    }
}

impl Whoami {
    pub fn new() -> Whoami {
        Whoami {
            version: 1,
            address: Address::new(),
            services: vec!["node".to_string()],
        }
    }
}

impl Serialize for Whoami {
    fn serialize(&self) -> Vec<u8> {
        let mut v = Vec::new();
        v.append(&mut self.version.serialize());
        v.append(&mut self.address.serialize());
        v.append(&mut self.services.serialize());
        v
    }
}

pub struct WhoamiAck {}

impl WhoamiAck {
    pub fn new() -> WhoamiAck {
        WhoamiAck {}
    }
}

impl Serialize for WhoamiAck {
    fn serialize(&self) -> Vec<u8> {
        Vec::new()
    }
}

impl Message for WhoamiAck {
    fn message_type() -> [u8; 12] {
        [119, 104, 111, 97, 109, 105, 97, 99, 107, 0, 0, 0]
    }
}

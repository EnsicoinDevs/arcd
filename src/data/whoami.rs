use crate::data::{Message, MessageType};
use crate::network::Address;
use ensicoin_serializer::Result as DeserResult;
use ensicoin_serializer::{Deserialize, Deserializer, Serialize};

pub struct Whoami {
    version: u32,
    address: Address,
    services: Vec<String>,
}

impl Message for Whoami {
    fn message_string() -> [u8; 12] {
        [119, 104, 111, 97, 109, 105, 0, 0, 0, 0, 0, 0]
    }
    fn message_type() -> MessageType {
        MessageType::Whoami
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

impl Deserialize for Whoami {
    fn deserialize(de: &mut Deserializer) -> DeserResult<Whoami> {
        let version = u32::deserialize(de)?;
        let address = Address::deserialize(de)?;
        let services: Vec<String> = Vec::deserialize(de)?;

        Ok(Whoami {
            version,
            address,
            services,
        })
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
    fn message_string() -> [u8; 12] {
        [119, 104, 111, 97, 109, 105, 97, 99, 107, 0, 0, 0]
    }
    fn message_type() -> MessageType {
        MessageType::WhoamiAck
    }
}

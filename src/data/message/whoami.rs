use super::{Message, MessageType};
use crate::network::Address;
use ensicoin_serializer::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Whoami {
    pub version: u32,
    pub address: Address,
    pub services: Vec<String>,
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

#[derive(Serialize, Deserialize)]
pub struct WhoamiAck {}

impl WhoamiAck {
    pub fn new() -> WhoamiAck {
        WhoamiAck {}
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

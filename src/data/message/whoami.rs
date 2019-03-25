use super::{Message, MessageType};
use crate::network::Address;
use ensicoin_serializer::Result as DeserResult;
use ensicoin_serializer::{Deserialize, Deserializer, Serialize};

#[derive(Serialize)]
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

impl Deserialize for Whoami {
    fn deserialize(de: &mut Deserializer) -> DeserResult<Whoami> {
        let version = match u32::deserialize(de) {
            Ok(x) => x,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "In Whoami reading version: {}",
                    e
                )));
            }
        };
        let address = match Address::deserialize(de) {
            Ok(x) => x,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "In Whoami reading address: {}",
                    e
                )));
            }
        };
        let services: Vec<String> = match Vec::deserialize(de) {
            Ok(x) => x,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "In Whoami reading services: {}",
                    e
                )));
            }
        };

        Ok(Whoami {
            version,
            address,
            services,
        })
    }
}

#[derive(Serialize)]
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

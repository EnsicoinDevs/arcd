use ensicoin_serializer::types::Sha256Result;
use ensicoin_serializer::{Deserialize, Result, Serialize};

use super::message::{Message, MessageType};

#[derive(Serialize)]
pub struct GetBlocks {
    block_locator: Vec<Sha256Result>,
    stop_hash: Sha256Result,
}

impl Message for GetBlocks {
    fn message_string() -> [u8; 12] {
        [103, 101, 116, 98, 108, 111, 99, 107, 115, 0, 0, 0]
    }
    fn message_type() -> MessageType {
        MessageType::GetBlocks
    }
}

impl Deserialize for GetBlocks {
    fn deserialize(de: &mut ensicoin_serializer::Deserializer) -> Result<GetBlocks> {
        let block_locator = match Vec::deserialize(de) {
            Ok(inventory) => inventory,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading GetBlocks block_locator: {}",
                    e
                )));
            }
        };
        let stop_hash = match Sha256Result::deserialize(de) {
            Ok(hash) => hash,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading GetBlocks stop_hash: {}",
                    e
                )));
            }
        };
        Ok(GetBlocks {
            block_locator,
            stop_hash,
        })
    }
}

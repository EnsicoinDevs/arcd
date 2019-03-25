use ensicoin_serializer::types::Sha256Result;
use ensicoin_serializer::{Deserialize, Serialize};

use super::message::{Message, MessageType};

#[derive(Serialize, Deserialize)]
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

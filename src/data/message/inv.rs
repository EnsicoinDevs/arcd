use ensicoin_serializer::types::Sha256Result;
use ensicoin_serializer::{Deserialize, Serialize};

use crate::data::message::{Message, MessageType};

#[derive(Serialize, Deserialize)]
pub struct InvVect {
    data_type: super::message::DataType,
    hash: Sha256Result,
}

#[derive(Serialize, Deserialize)]
pub struct Inv {
    inventory: Vec<InvVect>,
}

impl Message for Inv {
    fn message_string() -> [u8; 12] {
        [105, 110, 118, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    }
    fn message_type() -> MessageType {
        MessageType::Inv
    }
}

#[derive(Serialize, Deserialize)]
pub struct GetData {
    inventory: Vec<InvVect>,
}

impl Message for GetData {
    fn message_string() -> [u8; 12] {
        [103, 101, 116, 100, 97, 116, 97, 0, 0, 0, 0, 0]
    }
    fn message_type() -> MessageType {
        MessageType::GetData
    }
}

#[derive(Serialize, Deserialize)]
pub struct NotFound {
    inventory: Vec<InvVect>,
}

impl Message for NotFound {
    fn message_string() -> [u8; 12] {
        [110, 111, 116, 102, 111, 117, 110, 100, 0, 0, 0, 0]
    }
    fn message_type() -> MessageType {
        MessageType::NotFound
    }
}

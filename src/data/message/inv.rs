use ensicoin_serializer::types::Sha256Result;
use ensicoin_serializer::{Deserialize, Result, Serialize};

use super::message::DataType;

use crate::data::message::{Message, MessageType};

pub struct InvVect {
    data_type: super::message::DataType,
    hash: Sha256Result,
}

impl Deserialize for InvVect {
    fn deserialize(de: &mut ensicoin_serializer::Deserializer) -> Result<InvVect> {
        let data_type = match DataType::deserialize(de) {
            Ok(t) => t,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading Inv_vect type: {}",
                    e
                )));
            }
        };
        let hash = match Sha256Result::deserialize(de) {
            Ok(h) => h,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading Inv_vect hash: {}",
                    e
                )));
            }
        };
        Ok(InvVect { data_type, hash })
    }
}

impl Serialize for InvVect {
    fn serialize(&self) -> Vec<u8> {
        let mut v = self.data_type.serialize();
        v.append(&mut self.hash.serialize());
        v
    }
}

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

impl Deserialize for Inv {
    fn deserialize(de: &mut ensicoin_serializer::Deserializer) -> Result<Inv> {
        match Vec::deserialize(de) {
            Ok(inventory) => Ok(Inv { inventory }),
            Err(e) => Err(ensicoin_serializer::Error::Message(format!(
                "Error in reading Inv: {}",
                e
            ))),
        }
    }
}

impl Serialize for Inv {
    fn serialize(&self) -> Vec<u8> {
        self.inventory.serialize()
    }
}

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

impl Deserialize for GetData {
    fn deserialize(de: &mut ensicoin_serializer::Deserializer) -> Result<GetData> {
        match Vec::deserialize(de) {
            Ok(inventory) => Ok(GetData { inventory }),
            Err(e) => Err(ensicoin_serializer::Error::Message(format!(
                "Error in reading GetData: {}",
                e
            ))),
        }
    }
}

impl Serialize for GetData {
    fn serialize(&self) -> Vec<u8> {
        self.inventory.serialize()
    }
}

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

impl Deserialize for NotFound {
    fn deserialize(de: &mut ensicoin_serializer::Deserializer) -> Result<NotFound> {
        match Vec::deserialize(de) {
            Ok(inventory) => Ok(NotFound { inventory }),
            Err(e) => Err(ensicoin_serializer::Error::Message(format!(
                "Error in reading NotFound: {}",
                e
            ))),
        }
    }
}

impl Serialize for NotFound {
    fn serialize(&self) -> Vec<u8> {
        self.inventory.serialize()
    }
}

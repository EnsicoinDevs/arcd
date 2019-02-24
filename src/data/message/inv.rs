extern crate ensicoin_serializer;

use ensicoin_serializer::types::Hash;
use ensicoin_serializer::{Deserialize, Result, Serialize};

use super::message::DataType;

pub struct InvVect {
    data_type: super::message::DataType,
    hash: Hash,
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
        let hash = match Hash::deserialize(de) {
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

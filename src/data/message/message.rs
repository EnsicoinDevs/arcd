extern crate ensicoin_serializer;
use crate::network;
use ensicoin_serializer::{Deserialize, Serialize};

pub enum DataType {
    Transaction,
    Block,
}

impl Serialize for DataType {
    fn serialize(&self) -> Vec<u8> {
        match self {
            DataType::Block => (1 as u32).serialize(),
            DataType::Transaction => (0 as u32).serialize(),
        }
    }
}

impl Deserialize for DataType {
    fn deserialize(
        de: &mut ensicoin_serializer::Deserializer,
    ) -> ensicoin_serializer::Result<DataType> {
        match u32::deserialize(de) {
            Ok(0) => Ok(DataType::Transaction),
            Ok(1) => Ok(DataType::Block),
            Ok(n) => Err(ensicoin_serializer::Error::Message(format!(
                "Invalid DataType: {}",
                n
            ))),
            Err(e) => Err(ensicoin_serializer::Error::Message(format!(
                "Error reading DataType: {}",
                e
            ))),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum MessageType {
    Whoami,
    WhoamiAck,
    Inv,
    GetData,
    NotFound,
    GetBlocks,
    Transaction,
    Ping,
    Pong,
    Unknown(String),
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MessageType::Ping => "2plus2is4".to_string(),
                MessageType::Pong => "minus1thats3".to_string(),
                MessageType::Whoami => "Whoami".to_string(),
                MessageType::WhoamiAck => "WhoamiAck".to_string(),
                MessageType::Inv => "Inv".to_string(),
                MessageType::GetData => "GetData".to_string(),
                MessageType::NotFound => "NotFound".to_string(),
                MessageType::GetBlocks => "GetBlocks".to_string(),
                MessageType::Transaction => "Transaction".to_string(),
                MessageType::Unknown(s) => {
                    format!("Unknown: {}", s).trim_matches('\x00').to_string()
                }
            }
        )
    }
}

pub trait Message: Serialize {
    fn message_string() -> [u8; 12];
    fn message_type() -> MessageType;
    fn raw_bytes(&self) -> Result<(MessageType, Vec<u8>), network::Error> {
        let magic: u32 = 422021;
        let message_string = Self::message_string();
        let mut payload = self.serialize();
        let payload_length: u64 = payload.len() as u64;

        let mut v = Vec::new();
        v.append(&mut magic.serialize());
        v.extend_from_slice(&message_string);
        v.append(&mut payload_length.serialize());
        v.append(&mut payload);
        Ok((Self::message_type(), v))
    }
}

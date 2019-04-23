use bytes::{Bytes, BytesMut};
use ensicoin_serializer::{Deserialize, Deserializer, Serialize};

use tokio::codec::{Decoder, Encoder};

pub struct MessageCodec {
    decoding_payload: bool,
    message_type: MessageType,
    payload_size: usize,
}

impl MessageCodec {
    pub fn new() -> MessageCodec {
        MessageCodec {
            decoding_payload: false,
            message_type: MessageType::Unknown(Vec::new()),
            payload_size: 0,
        }
    }
}

impl Decoder for MessageCodec {
    type Item = (MessageType, BytesMut);
    type Error = crate::network::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if !self.decoding_payload && buf.len() >= 24 {
            trace!("Reading header");
            let header = buf.split_to(24);
            let mut de = Deserializer::new(header);

            let magic = u32::deserialize(&mut de).unwrap_or(0);
            if magic != crate::constants::MAGIC {
                return Err(crate::network::Error::InvalidMagic(magic));
            };
            let message_type = MessageType::deserialize(&mut de).unwrap();
            let payload_length = u64::deserialize(&mut de).unwrap_or(0) as usize;
            self.decoding_payload = true;
            self.message_type = message_type;
            self.payload_size = payload_length;
            trace!(
                "message: {} of size {} to read",
                self.message_type,
                self.payload_size
            );
        }
        if self.decoding_payload && buf.len() >= self.payload_size {
            trace!("Reading payload");
            self.decoding_payload = false;
            Ok(Some((
                self.message_type.clone(),
                buf.split_to(self.payload_size),
            )))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for MessageCodec {
    type Item = Bytes;
    type Error = crate::network::Error;

    fn encode(
        &mut self,
        raw_message: Self::Item,
        buf: &mut BytesMut,
    ) -> Result<(), crate::network::Error> {
        buf.extend_from_slice(&raw_message);
        Ok(())
    }
}

pub enum DataType {
    Transaction,
    Block,
}

impl Serialize for DataType {
    fn serialize(&self) -> Bytes {
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

#[derive(PartialEq, Eq, Debug, Clone)]
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
    Unknown(Vec<u8>),
}

impl Deserialize for MessageType {
    fn deserialize(de: &mut Deserializer) -> ensicoin_serializer::Result<MessageType> {
        let raw_type = match de.extract_bytes(12) {
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading message type: {}",
                    e
                )));
            }
            Ok(v) => v.to_vec(),
        };
        Ok(
            if raw_type == [119, 104, 111, 97, 109, 105, 0, 0, 0, 0, 0, 0] {
                MessageType::Whoami
            } else if raw_type == [119, 104, 111, 97, 109, 105, 97, 99, 107, 0, 0, 0] {
                MessageType::WhoamiAck
            } else if raw_type == [50, 112, 108, 117, 115, 50, 105, 115, 52, 0, 0, 0] {
                MessageType::Ping
            } else if raw_type == [109, 105, 110, 117, 115, 49, 116, 104, 97, 116, 115, 51] {
                MessageType::Pong
            } else if raw_type == [105, 110, 118, 0, 0, 0, 0, 0, 0, 0, 0, 0] {
                MessageType::Inv
            } else if raw_type == [103, 101, 116, 100, 97, 116, 97, 0, 0, 0, 0, 0] {
                MessageType::GetData
            } else if raw_type == [110, 111, 116, 102, 111, 117, 110, 100, 0, 0, 0, 0] {
                MessageType::NotFound
            } else if raw_type == [103, 101, 116, 98, 108, 111, 99, 107, 115, 0, 0, 0] {
                MessageType::GetBlocks
            } else if raw_type == [116, 120, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] {
                MessageType::Transaction
            } else {
                MessageType::Unknown(raw_type)
            },
        )
    }
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
                MessageType::Unknown(s) => format!(
                    "Unknown: {}",
                    String::from_utf8(s.clone()).unwrap_or("<INVALID UTF8>".to_string())
                )
                .trim_matches('\x00')
                .to_string(),
            }
        )
    }
}

pub trait Message: Serialize {
    fn message_string() -> [u8; 12];
    fn message_type() -> MessageType;
    fn raw_bytes(&self) -> (MessageType, Bytes) {
        let magic: u32 = 422021;
        let message_string = Self::message_string();
        let payload = self.serialize();
        let payload_length: u64 = payload.len() as u64;

        let mut v = Bytes::new();
        v.extend_from_slice(&magic.serialize());
        v.extend_from_slice(&message_string);
        v.extend_from_slice(&payload_length.serialize());
        v.extend_from_slice(&payload);
        (Self::message_type(), v)
    }
}

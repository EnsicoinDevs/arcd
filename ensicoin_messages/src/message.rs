use bytes::{Bytes, BytesMut};
use ensicoin_serializer::{Deserialize, Deserializer, Serialize, Sha256Result};

pub use super::resource::{Block, Transaction};

#[derive(Serialize, Deserialize, Clone)]
pub struct GetBlocks {
    pub block_locator: Vec<Sha256Result>,
    pub stop_hash: Sha256Result,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Address {
    pub timestamp: u64,
    pub ip: [u8; 16],
    pub port: u16,
}

impl Deserialize for Address {
    fn deserialize(de: &mut Deserializer) -> Result<Self, ensicoin_serializer::Error> {
        let timestamp = u64::deserialize(de)?;
        let ip_bytes = de.extract_bytes(16)?;
        let mut ip = [0; 16];
        for (i, b) in ip_bytes.iter().enumerate() {
            ip[i] = *b;
        }
        let port = u16::deserialize(de)?;

        Ok(Address {
            timestamp,
            ip,
            port,
        })
    }
}

impl Serialize for Address {
    fn serialize(&self) -> Bytes {
        let mut buf = Bytes::new();
        buf.extend_from_slice(&self.timestamp.serialize());
        buf.extend_from_slice(&self.ip);
        buf.extend_from_slice(&self.port.serialize());
        buf
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Whoami {
    pub version: u32,
    pub address: Address,
    pub services: Vec<String>,
}

impl Whoami {
    pub fn new(address: Address) -> Whoami {
        Whoami {
            version: 1,
            address,
            services: vec!["node".to_string()],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct InvVect {
    pub data_type: crate::message::ResourceType,
    pub hash: Sha256Result,
}

impl std::fmt::Debug for InvVect {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ensicoin_serializer::hash_to_string;
        f.debug_struct("InvVect")
            .field("data_type", &self.data_type)
            .field("hash", &hash_to_string(&self.hash))
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ResourceType {
    Transaction,
    Block,
}

impl Serialize for ResourceType {
    fn serialize(&self) -> Bytes {
        match self {
            ResourceType::Block => (1 as u32).serialize(),
            ResourceType::Transaction => (0 as u32).serialize(),
        }
    }
}

impl Deserialize for ResourceType {
    fn deserialize(
        de: &mut ensicoin_serializer::Deserializer,
    ) -> ensicoin_serializer::Result<ResourceType> {
        match u32::deserialize(de) {
            Ok(0) => Ok(ResourceType::Transaction),
            Ok(1) => Ok(ResourceType::Block),
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
    GetMempool,
    GetAddr,
    Addr,
    Block,
    Transaction,
    Ping,
    Pong,
    Unknown(Vec<u8>),
}

//#[derive(Debug)] TODO add pretty debug to GetAddr & GetBlocks
#[derive(Clone)]
pub enum Message {
    Whoami(Whoami),
    WhoamiAck,
    GetAddr,
    Addr(Vec<Address>),
    GetMempool,
    GetBlocks(GetBlocks),
    Inv(Vec<InvVect>),
    GetData(Vec<InvVect>),
    NotFound(Vec<InvVect>),
    Ping,
    Pong,
    Block(Block),
    Tx(Transaction),
}

#[derive(Deserialize, Clone)]
pub struct MessageHeader {
    pub magic: u32,
    pub message_type: MessageType,
    pub payload_length: u64,
}
impl MessageHeader {
    pub fn from_bytes(expected_magic: u32, bytes: BytesMut) -> Result<Self, MessageError> {
        let mut de = ensicoin_serializer::Deserializer::new(bytes);
        let header = Self::deserialize(&mut de)?;
        if header.magic != expected_magic {
            Err(MessageError::InvalidMagic {
                expected: expected_magic,
                got: header.magic,
            })
        } else {
            Ok(header)
        }
    }
}

#[derive(Debug)]
pub enum MessageError {
    InvalidMagic { expected: u32, got: u32 },
    UnknownType(Vec<u8>),
    InvalidSize { expected: u64, got: u64 },
    InvalidPayload(ensicoin_serializer::Error),
}
impl From<ensicoin_serializer::Error> for MessageError {
    fn from(err: ensicoin_serializer::Error) -> Self {
        Self::InvalidPayload(err)
    }
}

impl Message {
    pub fn from_payload(header: MessageHeader, payload: BytesMut) -> Result<Message, MessageError> {
        if header.payload_length != payload.len() as u64 {
            return Err(MessageError::InvalidSize {
                expected: header.payload_length,
                got: payload.len() as u64,
            });
        }
        let mut de = ensicoin_serializer::Deserializer::new(payload);
        Ok(match header.message_type {
            MessageType::WhoamiAck => Message::WhoamiAck,
            MessageType::Ping => Message::Ping,
            MessageType::Pong => Message::Pong,
            MessageType::GetMempool => Message::GetMempool,
            MessageType::Whoami => Message::Whoami(Whoami::deserialize(&mut de)?),
            MessageType::GetAddr => Message::GetAddr,
            MessageType::Addr => Message::Addr(Vec::deserialize(&mut de)?),
            MessageType::GetBlocks => Message::GetBlocks(GetBlocks::deserialize(&mut de)?),
            MessageType::Inv => Message::Inv(Vec::deserialize(&mut de)?),
            MessageType::GetData => Message::GetData(Vec::deserialize(&mut de)?),
            MessageType::NotFound => Message::NotFound(Vec::deserialize(&mut de)?),
            MessageType::Block => Message::Block(Block::deserialize(&mut de)?),
            MessageType::Transaction => Message::Tx(Transaction::deserialize(&mut de)?),
            MessageType::Unknown(v) => return Err(MessageError::UnknownType(v)),
        })
    }
    pub fn message_type(&self) -> MessageType {
        match self {
            Message::Whoami(_) => MessageType::Whoami,
            Message::WhoamiAck => MessageType::WhoamiAck,
            Message::GetAddr => MessageType::GetAddr,
            Message::Addr(_) => MessageType::Addr,
            Message::GetMempool => MessageType::GetMempool,
            Message::GetBlocks(_) => MessageType::GetBlocks,
            Message::Inv(_) => MessageType::Inv,
            Message::GetData(_) => MessageType::GetData,
            Message::NotFound(_) => MessageType::NotFound,
            Message::Ping => MessageType::Ping,
            Message::Pong => MessageType::Pong,
            Message::Block(_) => MessageType::Block,
            Message::Tx(_) => MessageType::Transaction,
        }
    }
    pub fn payload(&self) -> Bytes {
        match self {
            Message::Ping
            | Message::Pong
            | Message::GetMempool
            | Message::WhoamiAck
            | Message::GetAddr => Bytes::new(),
            Message::Whoami(m) => m.serialize(),
            Message::Addr(m) => m.serialize(),
            Message::GetBlocks(m) => m.serialize(),
            Message::Inv(m) => m.serialize(),
            Message::GetData(m) => m.serialize(),
            Message::NotFound(m) => m.serialize(),
            Message::Block(m) => m.serialize(),
            Message::Tx(m) => m.serialize(),
        }
    }

    pub fn as_bytes(&self, magic: u32) -> Bytes {
        let mut bytes = bytes::BytesMut::new();
        bytes.extend_from_slice(magic.serialize().as_ref());
        bytes.extend_from_slice(self.message_type().serialize().as_ref());
        let payload = self.payload();
        bytes.extend_from_slice((payload.len() as u64).serialize().as_ref());
        bytes.extend_from_slice(payload.as_ref());
        Bytes::from(bytes)
    }
}

impl Serialize for MessageType {
    fn serialize(&self) -> Bytes {
        Bytes::from(
            &match self {
                MessageType::Whoami => [119, 104, 111, 97, 109, 105, 0, 0, 0, 0, 0, 0],
                MessageType::WhoamiAck => [119, 104, 111, 97, 109, 105, 97, 99, 107, 0, 0, 0],
                MessageType::GetAddr => [103, 101, 116, 97, 100, 100, 114, 0, 0, 0, 0, 0],
                MessageType::Addr => [97, 100, 100, 114, 0, 0, 0, 0, 0, 0, 0, 0],
                MessageType::GetBlocks => [103, 101, 116, 98, 108, 111, 99, 107, 115, 0, 0, 0],
                MessageType::GetMempool => [103, 101, 116, 109, 101, 109, 112, 111, 111, 108, 0, 0],
                MessageType::Inv => [105, 110, 118, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                MessageType::GetData => [103, 101, 116, 100, 97, 116, 97, 0, 0, 0, 0, 0],
                MessageType::NotFound => [110, 111, 116, 102, 111, 117, 110, 100, 0, 0, 0, 0],
                MessageType::Ping => [50, 112, 108, 117, 115, 50, 105, 115, 52, 0, 0, 0],
                MessageType::Pong => [109, 105, 110, 117, 115, 49, 116, 104, 97, 116, 115, 51],
                MessageType::Block => [98, 108, 111, 99, 107, 0, 0, 0, 0, 0, 0, 0],
                MessageType::Transaction => [116, 120, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                MessageType::Unknown(_) => [0; 12],
            }[..],
        )
    }
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
            } else if raw_type == [98, 108, 111, 99, 107, 0, 0, 0, 0, 0, 0, 0] {
                MessageType::Block
            } else if raw_type == [103, 101, 116, 97, 100, 100, 114, 0, 0, 0, 0, 0] {
                MessageType::GetAddr
            } else if raw_type == [103, 101, 116, 109, 101, 109, 112, 111, 111, 108, 0, 0] {
                MessageType::GetMempool
            } else if raw_type == [97, 100, 100, 114, 0, 0, 0, 0, 0, 0, 0, 0] {
                MessageType::Addr
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
                MessageType::GetMempool => "GetMempool".to_string(),
                MessageType::Transaction => "Transaction".to_string(),
                MessageType::GetAddr => "GetAddr".to_string(),
                MessageType::Addr => "Addr".to_string(),
                MessageType::Block => "Block".to_string(),
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

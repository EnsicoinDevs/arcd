use bytes::{Bytes, BytesMut};
use ensicoin_serializer::serializer::{fn_list, fn_str};
use ensicoin_serializer::{Deserialize, Deserializer, Sha256Result};

use cookie_factory::{
    bytes::{be_u16, be_u32, be_u64},
    combinator::{cond, slice},
    sequence::tuple,
    SerializeFn,
};
use std::io::Write;

pub use super::resource::{Block, Transaction, fn_tx, fn_block};

#[derive(Deserialize, Clone)]
pub struct GetBlocks {
    pub block_locator: Vec<Sha256Result>,
    pub stop_hash: Sha256Result,
}

pub fn fn_getblocks<'c, 'a: 'c, W: Write + 'c>(value: &'a GetBlocks) -> impl SerializeFn<W> + 'c {
    tuple((
        fn_list(
            value.block_locator.len() as u64,
            value.block_locator.iter().map(slice),
        ),
        slice(value.stop_hash),
    ))
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Address {
    pub timestamp: u64,
    pub ip: [u8; 16],
    pub port: u16,
}

pub fn fn_address<'c, W: Write + 'c>(address: Address) -> impl SerializeFn<W> + 'c {
    tuple((
        be_u64(address.timestamp),
        slice(address.ip),
        be_u16(address.port),
    ))
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

#[derive(Deserialize, Debug, Clone)]
pub struct Whoami {
    pub version: u32,
    pub address: Address,
    pub services: Vec<String>,
}

pub fn fn_whoami<'c, 'a: 'c, W: Write + 'c>(message: &'a Whoami) -> impl SerializeFn<W> + 'c {
    tuple((
        be_u32(message.version),
        fn_address(message.address),
        fn_list(
            message.services.len() as u64,
            message.services.iter().map(fn_str),
        ),
    ))
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

#[derive(Deserialize, Clone, Copy)]
pub struct InvVect {
    pub data_type: crate::message::ResourceType,
    pub hash: Sha256Result,
}

pub fn fn_inv_vect<'c, 'a: 'c, W: Write + 'c>(inv_vect: &'a InvVect) -> impl SerializeFn<W> + 'c {
    tuple((fn_res_type(inv_vect.data_type), slice(inv_vect.hash)))
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

pub fn fn_res_type<'c, W: Write + 'c>(res_type: ResourceType) -> impl SerializeFn<W> + 'c {
    be_u32(match res_type {
        ResourceType::Block => 1,
        ResourceType::Transaction => 0,
    })
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
    Block(Box<Block>),
    Tx(Box<Transaction>),
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
            MessageType::Block => Message::Block(Box::new(Block::deserialize(&mut de)?)),
            MessageType::Transaction => Message::Tx(Box::new(Transaction::deserialize(&mut de)?)),
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
}

pub fn fn_payload<'c, 'a: 'c, W: Write + 'c>(message: &'a Message) -> impl SerializeFn<W> + 'c {
    let msg_type = message.message_type();
    tuple((
        cond(
            msg_type == MessageType::Whoami,
            fn_whoami(match message {
                Message::Whoami(m) => m,
                _ => unreachable!(),
            }),
        ),
        cond(msg_type == MessageType::Addr, {
            let addr = match message {
                Message::Addr(a) => a,
                _ => unreachable!(),
            };
            fn_list(addr.len() as u64, addr.iter().map(|a| fn_address(*a)))
        }),
        cond(msg_type == MessageType::GetBlocks, {
            fn_getblocks(match message {
                Message::GetBlocks(g) => g,
                _ => unreachable!(),
            })
        }),
        cond(
            msg_type == MessageType::Inv
                || msg_type == MessageType::GetData
                || msg_type == MessageType::NotFound,
            {
                let vec = match message {
                    Message::Inv(v) | Message::GetData(v) | Message::NotFound(v) => v,
                    _ => unreachable!(),
                };
                fn_list(vec.len() as u64, vec.iter().map(fn_inv_vect))
            },
        ),
        cond(msg_type == MessageType::Block, {
            fn_block(match message {
                Message::Block(b) => b,
                _ => unreachable!(),
            })
        }),
        cond(msg_type == MessageType::Transaction, {
            fn_tx(match message {
                Message::Tx(t) => t,
                _ => unreachable!(),
            })
        }),
    ))
}

pub fn fn_message<'c, 'a: 'c, W: Write + 'c>(
    message: &'a Message,
    magic: u32,
) -> impl SerializeFn<W> + 'c {
    let payload = cookie_factory::gen_simple(fn_payload(message), Vec::new()).expect("payload");
    tuple((
        be_u32(magic),
        fn_message_type(message.message_type()),
        be_u64(payload.len() as u64),
        slice(payload),
    ))
}

pub fn fn_message_type<'c, W: Write + 'c>(msg_type: MessageType) -> impl SerializeFn<W> + 'c {
    slice(match msg_type {
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
    })
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

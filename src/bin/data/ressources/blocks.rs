use ensicoin_serializer::types::Sha256Result;
use ensicoin_serializer::{Deserialize, Serialize};

use sha2::Digest;

use crate::data::message::{Message, MessageType};
use crate::data::ressources::Transaction;

#[derive(Serialize, Deserialize)]
pub struct BlockHeader {
    version: u32,
    flags: Vec<String>,
    pub prev_block: Sha256Result,
    merkle_root: Sha256Result,
    timestamp: u64,
    pub height: u32,
    bits: u32,
    nonce: u64,
}

#[derive(Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    txs: Vec<Transaction>,
}

impl Block {
    pub fn double_hash(&self) -> Sha256Result {
        let bytes = self.serialize();
        let mut hasher = sha2::Sha256::default();
        hasher.input(bytes);
        hasher.result()
    }
    pub fn utxo(&self) -> Vec<crate::data::ressources::Outpoint> {
        Vec::new()
    }
}

impl Message for Block {
    fn message_string() -> [u8; 12] {
        [98, 108, 111, 99, 107, 0, 0, 0, 0, 0, 0, 0]
    }

    fn message_type() -> MessageType {
        MessageType::Block
    }
}

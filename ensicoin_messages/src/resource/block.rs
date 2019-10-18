use ensicoin_serializer::{hash_to_string, types::Sha256Result, Deserialize, Serialize};

use sha2::Digest;

use crate::resource::Transaction;

#[derive(Serialize, Deserialize, Clone)]
pub struct BlockHeader {
    pub version: u32,
    pub flags: Vec<String>,
    pub prev_block: Sha256Result,
    pub merkle_root: Sha256Result,
    pub timestamp: u64,
    pub height: u32,
    pub target: Sha256Result,
    pub nonce: u64,
}

impl std::fmt::Debug for BlockHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("BlockHeader")
            .field("version", &self.version)
            .field("flags", &self.flags)
            .field("prev_block", &hash_to_string(&self.prev_block))
            .field("merkle_root", &hash_to_string(&self.merkle_root))
            .field("timestamp", &self.timestamp)
            .field("height", &self.height)
            .field("target", &hash_to_string(&self.target))
            .field("nonce", &self.nonce)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub txs: Vec<Transaction>,
}

impl BlockHeader {
    pub fn double_hash(&self) -> Sha256Result {
        let bytes = self.serialize();
        let mut hasher = sha2::Sha256::default();
        hasher.input(bytes);
        let first = hasher.result();
        hasher = sha2::Sha256::default();
        hasher.input(first);
        hasher.result()
    }
}

impl Block {
    pub fn double_hash(&self) -> Sha256Result {
        self.header.double_hash()
    }
}

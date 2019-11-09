use ensicoin_serializer::{
    hash_to_string,
    serializer::{fn_list, fn_str},
    types::Sha256Result,
    Deserialize,
};

use sha2::Digest;

use super::tx::fn_tx;
use crate::resource::Transaction;
use cookie_factory::{
    bytes::{be_u32, be_u64},
    combinator::slice,
    sequence::tuple,
    SerializeFn,
};
use std::io::Write;

#[derive(Deserialize, Clone)]
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

pub fn fn_block_header<'c, 'a: 'c, W: Write + 'c>(
    header: &'a BlockHeader,
) -> impl SerializeFn<W> + 'c {
    tuple((
        be_u32(header.version),
        fn_list(header.flags.len() as u64, header.flags.iter().map(fn_str)),
        slice(header.prev_block),
        slice(header.merkle_root),
        be_u64(header.timestamp),
        be_u32(header.height),
        slice(header.target),
        be_u64(header.nonce),
    ))
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

#[derive(Deserialize, Clone, Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub txs: Vec<Transaction>,
}

pub fn fn_block<'c, 'a: 'c, W: Write + 'c>(block: &'a Block) -> impl SerializeFn<W> + 'c {
    tuple((
        fn_block_header(&block.header),
        fn_list(block.txs.len() as u64, block.txs.iter().map(fn_tx)),
    ))
}

impl BlockHeader {
    pub fn serialize(&self) -> Vec<u8> {
        crate::as_bytes(fn_block_header(self))
    }

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

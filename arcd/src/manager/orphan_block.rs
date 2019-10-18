use crate::data::intern_messages::Source;
use ensicoin_messages::resource::Block;
use ensicoin_serializer::Sha256Result;
use std::collections::HashMap;

pub type OriginedBlock = (Source, Block);

pub struct OrphanBlockManager {
    storage: HashMap<Sha256Result, OriginedBlock>,
}

impl OrphanBlockManager {
    pub fn new() -> OrphanBlockManager {
        OrphanBlockManager {
            storage: HashMap::new(),
        }
    }

    pub fn add_block(&mut self, (source, block): OriginedBlock) {
        self.storage
            .insert(block.header.prev_block, (source, block));
    }

    pub fn retrieve_chain(&mut self, new_block_hash: Sha256Result) -> Vec<OriginedBlock> {
        let mut chain = Vec::new();
        let mut hash = new_block_hash;
        while let Some((s, b)) = self.storage.remove(&hash) {
            hash = b.double_hash();
            chain.push((s, b));
        }
        chain
    }
}

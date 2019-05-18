use crate::data::linkedtx::LinkedTransaction;
use ensicoin_messages::resource::{Block, BlockHeader};

pub struct LinkedBlock {
    pub header: BlockHeader,
    pub txs: Vec<LinkedTransaction>,
}

impl LinkedBlock {
    pub fn new(block: Block) -> LinkedBlock {
        let txs = block.txs;
        let header = block.header;
        LinkedBlock {
            header,
            txs: txs
                .into_iter()
                .map(|tx| LinkedTransaction::new(tx))
                .collect(),
        }
    }
    pub fn to_block(self) -> Block {
        let header = self.header;
        let txs = self.txs;
        Block {
            header: header,
            txs: txs.into_iter().map(|txs| txs.transaction).collect(),
        }
    }
    pub fn is_valid(&self) -> bool {
        // TODO: Get the target
        for tx in &self.txs[1..] {
            if !tx.is_complete() {
                return false;
            }
            match tx.is_valid() {
                Ok(true) => (),
                _ => return false,
            }
        }
        return true;
    }
}

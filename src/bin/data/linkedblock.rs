use crate::data::linkedtx::LinkedTransaction;
use ensicoin_messages::resource::{Block, BlockHeader};

pub struct LinkedBlock {
    header: BlockHeader,
    txs: Vec<LinkedTransaction>,
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
}

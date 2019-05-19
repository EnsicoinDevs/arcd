use crate::data::{linkedtx::LinkedTransaction, PairedUtxo};
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

    pub fn spent_utxo(&self) -> Vec<PairedUtxo> {
        let mut utxos = Vec::new();
        for ltx in &self.txs {
            utxos.append(&mut ltx.get_dependent_utxo());
        }
        utxos
    }

    pub fn work(&self) -> num_bigint::BigUint {
        num_bigint::BigUint::from_bytes_be(&[0xff; 32])
            - num_bigint::BigUint::from_bytes_be(&self.header.target)
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

use crate::data::{linkedtx::LinkedTransaction, PairedUtxo};
use ensicoin_messages::resource::{Block, BlockHeader};
use ensicoin_serializer::{hash_to_string, Sha256Result};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct LinkedBlock {
    pub header: BlockHeader,
    pub txs: Vec<LinkedTransaction>,
}

fn double_hash(hash_a: Sha256Result, hash_b: Sha256Result) -> Sha256Result {
    let mut hasher = Sha256::default();

    let mut vec_hash = hash_a.to_vec();
    vec_hash.extend_from_slice(&hash_b);
    hasher.input(vec_hash);
    let first = hasher.result();
    hasher = Sha256::default();
    hasher.input(first);

    hasher.result()
}

impl LinkedBlock {
    pub fn merkle_root(&self) -> Sha256Result {
        let mut resources: Vec<_> = self
            .txs
            .iter()
            .map(|tx| tx.transaction.double_hash())
            .collect();
        if resources.len() == 0 {
            return Sha256Result::from([0; 32]);
        };

        if resources.len() == 1 {
            resources.push(resources[0]);
        };

        while resources.len() > 1 {
            if resources.len() % 2 != 0 {
                resources.push(resources.last().unwrap().clone());
            }

            let mut left_hash = resources[0];
            for i in 0..resources.len() {
                let hash = resources[i];

                if i % 2 == 0 {
                    left_hash = hash;
                } else {
                    resources[((i + 1) / 2) - 1] = double_hash(left_hash, hash);
                }
            }

            resources.split_off(resources.len() / 2);
        }

        resources[0]
    }

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
    pub fn is_valid(&self, target: num_bigint::BigUint, previous_height: u32) -> bool {
        if self.header.height != previous_height + 1 {
            warn!(
                "Invalid height: expected {} got {}",
                previous_height + 1,
                self.header.height
            );
            return false;
        };
        if num_bigint::BigUint::from_bytes_be(&self.header.target) != target {
            warn!(
                "Invalid target, expected: {} got {}",
                target,
                num_bigint::BigUint::from_bytes_be(&self.header.target),
            );
            return false;
        };
        if self.header.merkle_root != self.merkle_root() {
            warn!(
                "Invalid merkle root, expected {} got {}",
                hash_to_string(&self.merkle_root()),
                hash_to_string(&self.header.merkle_root),
            );
            return false;
        }
        if self.txs[0].transaction.flags.len() == 0 {
            warn!("Coinbase has no flags");
            return false;
        };
        let coinbase_height: u32 = match self.txs[0].transaction.flags[0].parse() {
            Ok(n) => n,
            Err(_) => {
                warn!("Coinbase first flag is not the height");
                return false;
            }
        };
        if coinbase_height != self.header.height {
            return false;
        };
        for tx in &self.txs[1..] {
            if !tx.is_complete() {
                warn!("Tx is not complete");
                return false;
            }
            match tx.is_valid() {
                Ok(true) => (),
                _ => {
                    warn!("Invalid tx");
                    return false;
                }
            }
        }
        return true;
    }
}

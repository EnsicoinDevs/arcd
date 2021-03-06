use ensicoin_messages::resource::{Block, Transaction};

pub trait SanityCheck {
    fn sanity_check(&self) -> bool;
}

impl SanityCheck for Transaction {
    fn sanity_check(&self) -> bool {
        for output in self.outputs.iter() {
            if output.value == 0 {
                return false;
            }
        }
        !self.inputs.is_empty() && !self.outputs.is_empty()
    }
}

impl SanityCheck for Block {
    fn sanity_check(&self) -> bool {
        for tx in &self.txs {
            if !tx.sanity_check() {
                return false;
            }
        }
        let bytes_num = num_bigint::BigUint::from_bytes_be(&self.header.target);
        let hash_num = num_bigint::BigUint::from_bytes_be(&self.double_hash().to_vec());
        !self.txs.is_empty() && hash_num < bytes_num
    }
}

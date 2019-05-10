extern crate ensicoin_messages;
extern crate ensicoin_serializer;
extern crate sled;
use ensicoin_serializer::Sha256Result;

fn main() {
    let genesis = ensicoin_messages::resource::Block {
        header: ensicoin_messages::resource::BlockHeader {
            version: 0,
            flags: vec!["ici cest limag".to_string()],
            prev_block: Sha256Result::from([0; 32]),
            merkle_root: Sha256Result::from([0; 32]),
            timestamp: 1566862920,
            nonce: 42,
            height: 0,
            bits: 1,
        },
        txs: Vec::new(),
    };
}

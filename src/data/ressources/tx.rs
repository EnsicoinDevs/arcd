extern crate ensicoin_serializer;
use ensicoin_serializer::types::Hash;

pub struct Outpoint {
    hash: Hash,
    index: u32,
}

pub struct TransactionInput {
    previous_output: Outpoint,
}

use ensicoin_serializer::types::Sha256Result;
use ensicoin_serializer::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::script::Script;
use crate::data::message::{Message, MessageType};

#[derive(Hash, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Outpoint {
    pub hash: Sha256Result,
    pub index: u32,
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionInput {
    pub previous_output: Outpoint,
    script: Script,
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionOutput {
    value: u64,
    script: Script,
}

impl TransactionOutput {
    pub fn get_value(&self) -> &u64 {
        &self.value
    }
    pub fn get_script(&self) -> &Script {
        &self.script
    }
    pub fn get_data(&self, coinbase: bool, height: u32) -> crate::manager::UtxoData {
        crate::manager::UtxoData {
            script: self.script.clone(),
            value: self.value,
            coin_base: coinbase,
            block_height: height,
        }
    }
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    version: u32,
    flags: Vec<String>,
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
}

impl Transaction {
    pub fn get_data(
        &self,
        output_index: usize,
        coinbase: bool,
        height: u32,
    ) -> crate::manager::UtxoData {
        self.outputs[output_index].get_data(coinbase, height)
    }

    pub fn get_outputs(&self) -> &Vec<TransactionOutput> {
        &self.outputs
    }

    pub fn get_inputs(&self) -> &Vec<TransactionInput> {
        &self.inputs
    }
    pub fn double_hash(&self) -> Sha256Result {
        let bytes = self.serialize();
        let mut hasher = sha2::Sha256::default();
        hasher.input(bytes);
        hasher.result()
    }
}

impl Message for Transaction {
    fn message_string() -> [u8; 12] {
        [116, 120, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    }
    fn message_type() -> MessageType {
        MessageType::Transaction
    }
}

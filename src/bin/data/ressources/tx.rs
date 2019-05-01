use ensicoin_serializer::types::Sha256Result;
use ensicoin_serializer::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::script::Script;
use crate::data::message::{Message, MessageType};
use bytes::Bytes;

#[derive(PartialEq, Eq)]
pub struct UtxoData {
    pub script: Script,
    pub value: u64,
    pub block_height: u32,
    pub coin_base: bool,
}

impl Serialize for UtxoData {
    fn serialize(&self) -> Bytes {
        let mut v = self.script.serialize();
        v.extend_from_slice(&self.value.serialize());
        v.extend_from_slice(&self.block_height.serialize());
        v.extend_from_slice(&(self.coin_base as u8).serialize());
        v
    }
}

impl Deserialize for UtxoData {
    fn deserialize(
        de: &mut ensicoin_serializer::Deserializer,
    ) -> ensicoin_serializer::Result<Self> {
        let script = match Script::deserialize(de) {
            Ok(s) => s,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading UtxoData script: {}",
                    e
                )));
            }
        };
        let value = match u64::deserialize(de) {
            Ok(s) => s,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading UtxoData value: {}",
                    e
                )));
            }
        };
        let block_height = match u32::deserialize(de) {
            Ok(s) => s,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading UtxoData block_height: {}",
                    e
                )));
            }
        };
        let coin_base = match u8::deserialize(de) {
            Ok(0) => false,
            Ok(_) => true,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading UtxoData coin_base: {}",
                    e
                )));
            }
        };
        Ok(UtxoData {
            script,
            value,
            block_height,
            coin_base,
        })
    }
}

#[derive(
    Hash, Eq, PartialEq, Clone, Serialize, Deserialize, serde::Serialize, serde::Deserialize,
)]
pub struct Outpoint {
    pub hash: Sha256Result,
    pub index: u32,
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, serde::Serialize, serde::Deserialize)]
pub struct TransactionInput {
    pub previous_output: Outpoint,
    script: Script,
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, serde::Serialize, serde::Deserialize)]
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
    pub fn get_data(&self, coinbase: bool, height: u32) -> UtxoData {
        UtxoData {
            script: self.script.clone(),
            value: self.value,
            coin_base: coinbase,
            block_height: height,
        }
    }
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, serde::Serialize, serde::Deserialize)]
pub struct Transaction {
    version: u32,
    flags: Vec<String>,
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
}

impl Transaction {
    pub fn get_data(&self, output_index: usize, coinbase: bool, height: u32) -> UtxoData {
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

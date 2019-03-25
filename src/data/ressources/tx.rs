use ensicoin_serializer::types::Sha256Result;
use ensicoin_serializer::{Deserialize, Serialize};
use generic_array::GenericArray;
use sha2::{Digest, Sha256};

use super::script::Script;
use crate::data::message::{Message, MessageType};

#[derive(Hash, Eq, PartialEq, Clone, Serialize)]
pub struct Outpoint {
    pub hash: Sha256Result,
    pub index: u32,
}

impl Deserialize for Outpoint {
    fn deserialize(
        de: &mut ensicoin_serializer::Deserializer,
    ) -> ensicoin_serializer::Result<Outpoint> {
        let hash = match Sha256Result::deserialize(de) {
            Ok(h) => h,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading Outpoint hash: {}",
                    e
                )));
            }
        };
        let index = match u32::deserialize(de) {
            Ok(i) => i,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading Outpoint index: {}",
                    e
                )));
            }
        };
        Ok(Outpoint { hash, index })
    }
}

#[derive(Hash, PartialEq, Eq, Serialize)]
pub struct TransactionInput {
    pub previous_output: Outpoint,
    script: Script,
}

impl Deserialize for TransactionInput {
    fn deserialize(
        de: &mut ensicoin_serializer::Deserializer,
    ) -> ensicoin_serializer::Result<TransactionInput> {
        let previous_output = match Outpoint::deserialize(de) {
            Ok(p) => p,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading TransactionInput previous_output: {}",
                    e
                )));
            }
        };
        let script = match Script::deserialize(de) {
            Ok(s) => s,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading TransactionInput script: {}",
                    e
                )));
            }
        };
        Ok(TransactionInput {
            previous_output,
            script,
        })
    }
}

#[derive(Hash, PartialEq, Eq, Serialize)]
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

impl Deserialize for TransactionOutput {
    fn deserialize(
        de: &mut ensicoin_serializer::Deserializer,
    ) -> ensicoin_serializer::Result<TransactionOutput> {
        let value = match u64::deserialize(de) {
            Ok(v) => v,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading TransactionOutput value: {}",
                    e
                )));
            }
        };
        let script = match Script::deserialize(de) {
            Ok(s) => s,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading TransactionOutput script: {}",
                    e
                )));
            }
        };
        Ok(TransactionOutput { value, script })
    }
}

#[derive(Hash, PartialEq, Eq, Serialize)]
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

impl Deserialize for Transaction {
    fn deserialize(
        de: &mut ensicoin_serializer::Deserializer,
    ) -> ensicoin_serializer::Result<Transaction> {
        let version = match u32::deserialize(de) {
            Ok(v) => v,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading Transaction version: {}",
                    e
                )));
            }
        };
        let flags: Vec<String> = match Vec::deserialize(de) {
            Ok(f) => f,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading Transaction flags: {}",
                    e
                )));
            }
        };
        let inputs: Vec<TransactionInput> = match Vec::deserialize(de) {
            Ok(i) => i,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading Transaction inputs: {}",
                    e
                )));
            }
        };
        let outputs: Vec<TransactionOutput> = match Vec::deserialize(de) {
            Ok(o) => o,
            Err(e) => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Error in reading Transaction outputs: {}",
                    e
                )));
            }
        };
        Ok(Transaction {
            version,
            flags,
            inputs,
            outputs,
        })
    }
}

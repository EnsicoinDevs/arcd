use bytes::Bytes;
use ensicoin_messages::resource::{script::OP, tx::TransactionOutput};
use ensicoin_serializer::{Deserialize, Serialize};

#[derive(PartialEq, Eq)]
pub struct UtxoData {
    pub script: Vec<OP>,
    pub value: u64,
    pub block_height: u32,
    pub coin_base: bool,
}

impl UtxoData {
    pub fn from_output(output: &TransactionOutput, coinbase: bool, height: u32) -> UtxoData {
        UtxoData {
            script: output.script.clone(),
            value: output.value,
            coin_base: coinbase,
            block_height: height,
        }
    }
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
        let script: Vec<OP> = match Vec::deserialize(de) {
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

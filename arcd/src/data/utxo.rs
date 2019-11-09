use bytes::Bytes;
use ensicoin_messages::resource::{
    script::{fn_script, Script},
    tx::{fn_outpoint, Outpoint, TransactionOutput},
};
use ensicoin_serializer::Deserialize;

use cookie_factory::{
    bytes::{be_u32, be_u64, be_u8},
    sequence::tuple,
    SerializeFn,
};
use std::io::Write;

#[derive(Deserialize)]
pub struct PairedUtxo {
    pub data: UtxoData,
    pub outpoint: Outpoint,
}

pub fn ser_paired_utxo<'c, 'a: 'c, W: Write + 'c>(paired: &'a PairedUtxo) -> impl SerializeFn<W> + 'c {
    tuple((ser_utxo_data(&paired.data), fn_outpoint(&paired.outpoint)))
}

#[derive(PartialEq, Eq, Clone)]
pub struct UtxoData {
    pub script: Script,
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

pub fn ser_utxo_data<'c, 'a: 'c, W: Write + 'c>(data: &'a UtxoData) -> impl SerializeFn<W> + 'c {
    tuple((
        fn_script(&data.script),
        be_u64(data.value),
        be_u32(data.block_height),
        be_u8(data.coin_base as u8),
    ))
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

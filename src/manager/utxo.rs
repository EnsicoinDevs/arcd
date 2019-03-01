extern crate sled;

use crate::data::ressources::Script;
use crate::data::Outpoint;
use crate::data::Transaction;

extern crate ensicoin_serializer;
use ensicoin_serializer::{Deserialize, Serialize};

pub enum Error {
    DatabaseError(sled::Error<()>),
    ParseError(ensicoin_serializer::Error),
    NoValueFound,
}

impl From<sled::Error<()>> for Error {
    fn from(error: sled::Error<()>) -> Error {
        Error::DatabaseError(error)
    }
}

impl From<ensicoin_serializer::Error> for Error {
    fn from(error: ensicoin_serializer::Error) -> Error {
        Error::ParseError(error)
    }
}

pub struct UtxoManager {
    database: sled::Db,
}

impl UtxoManager {
    pub fn new(data_dir: &std::path::Path) -> UtxoManager {
        let mut utxo_dir = std::path::PathBuf::new();
        utxo_dir.push(data_dir);
        utxo_dir.push("utxo");
        let database = sled::Db::start_default(utxo_dir).unwrap();
        UtxoManager { database }
    }

    pub fn register(
        &self,
        tx: &Transaction,
        hash: &[u8],
        coin_base: bool,
        block_height: u32,
    ) -> Result<(), Error> {
        for (i, output) in tx.get_outputs().iter().enumerate() {
            let mut data = output.get_script().serialize();
            data.append(&mut output.get_value().serialize());
            data.append(&mut block_height.serialize());
            data.append(&mut (coin_base as u8).serialize());
            let outpoint = Outpoint {
                hash: ensicoin_serializer::types::Hash {
                    value: Vec::from(hash),
                },
                index: (i as u32),
            };
            self.database.set(outpoint.serialize(), data)?;
        }
        Ok(())
    }
    pub fn exists(&self, utxo: Outpoint) -> Result<bool, Error> {
        Ok(match self.database.get(utxo.serialize())? {
            Some(_) => true,
            None => false,
        })
    }

    pub fn get(&self, utxo: Outpoint) -> Result<UtxoData, Error> {
        match self.database.get(utxo.serialize())? {
            Some(x) => {
                let mut de = ensicoin_serializer::Deserializer::new(Vec::from(&*x));
                Ok(UtxoData::deserialize(&mut de)?)
            }
            None => Err(Error::NoValueFound),
        }
    }

    pub fn delete(&self, utxo: Outpoint) -> Result<(), Error> {
        self.database.del(utxo.serialize())?;
        Ok(())
    }
}

pub struct UtxoData {
    script: Script,
    value: u64,
    block_height: u32,
    coin_base: bool,
}

impl Serialize for UtxoData {
    fn serialize(&self) -> Vec<u8> {
        let mut v = self.script.serialize();
        v.append(&mut self.value.serialize());
        v.append(&mut self.block_height.serialize());
        v.append(&mut (self.coin_base as u8).serialize());
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

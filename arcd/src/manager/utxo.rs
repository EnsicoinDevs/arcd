use crate::{
    data::{linkedblock::LinkedBlock, ser_paired_utxo, ser_utxo_data, PairedUtxo, UtxoData},
    error::Error,
};
use bytes::BytesMut;
use ensicoin_messages::resource::tx::fn_outpoint;
use ensicoin_messages::resource::{Outpoint, Transaction};

use ensicoin_serializer::{Deserialize, Sha256Result};

pub struct UtxoManager {
    database: sled::Db,
}

impl UtxoManager {
    pub fn new(data_dir: &std::path::Path) -> UtxoManager {
        let mut utxo_dir = std::path::PathBuf::new();
        utxo_dir.push(data_dir);
        utxo_dir.push("utxo");
        let database = sled::Db::open(utxo_dir).unwrap();
        UtxoManager { database }
    }

    fn spend_block(&mut self, block: &LinkedBlock) -> Result<(), Error> {
        for pairedutxo in block.spent_utxo() {
            self.delete(&pairedutxo.outpoint)?;
        }
        Ok(())
    }

    pub fn register_block(&mut self, block: &LinkedBlock) -> Result<(), Error> {
        let height = block.header.height;
        let hash = block.header.double_hash();
        self.register(&block.txs[0].transaction, &hash, true, height)?;
        for tx in &block.txs[1..] {
            self.register(&tx.transaction, &hash, false, height)?;
        }
        self.spend_block(block)
    }

    pub fn register(
        &self,
        tx: &Transaction,
        hash: &[u8],
        coin_base: bool,
        block_height: u32,
    ) -> Result<(), Error> {
        for (i, output) in tx.outputs.iter().enumerate() {
            let data = ensicoin_messages::as_bytes(ser_utxo_data(&UtxoData {
                script: output.script.clone(),
                value: output.value,
                block_height,
                coin_base,
            }));
            let outpoint = Outpoint {
                hash: Sha256Result::clone_from_slice(hash),
                index: (i as u32),
            };
            self.database
                .insert(ensicoin_messages::as_bytes(fn_outpoint(&outpoint)), data)?;
        }
        Ok(())
    }

    pub fn get(&self, utxo: &Outpoint) -> Result<UtxoData, Error> {
        match self.database.get(ensicoin_messages::as_bytes(fn_outpoint(utxo)))? {
            Some(x) => {
                let mut de = ensicoin_serializer::Deserializer::new(BytesMut::from(&*x));
                Ok(UtxoData::deserialize(&mut de)?)
            }
            None => Err(Error::NotFound(format!("utxo {:?}", utxo.hash))),
        }
    }

    pub fn delete(&self, utxo: &Outpoint) -> Result<(), Error> {
        self.database.remove(ensicoin_messages::as_bytes(fn_outpoint(utxo)))?;
        Ok(())
    }

    pub fn link(&self, linkedtx: &mut crate::data::linkedtx::LinkedTransaction) {
        for parent in linkedtx.unknown().clone() {
            if let Ok(utxo) = self.get(&parent) {
                linkedtx.add_dependency(
                    parent.clone(),
                    crate::data::linkedtx::Dependency {
                        data: utxo,
                        dep_type: crate::data::linkedtx::DependencyType::Block,
                    },
                );
            }
        }
    }

    pub fn link_block(&self, linkedblock: &mut crate::data::linkedblock::LinkedBlock) {
        for ltx in linkedblock.txs.iter_mut() {
            self.link(ltx);
        }
    }

    pub fn restore(&mut self, utxos: Vec<PairedUtxo>) -> Result<(), Error> {
        for pairedtx in utxos {
            self.database.insert(
                ensicoin_messages::as_bytes(fn_outpoint(&pairedtx.outpoint)),
                ensicoin_messages::as_bytes(ser_utxo_data(&pairedtx.data)),
            )?;
        }
        Ok(())
    }
}

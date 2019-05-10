use crate::Error;
use ensicoin_messages::resource::Block;
use ensicoin_serializer::{Deserialize, Serialize, Sha256Result};

pub struct Blockchain {
    stats: sled::Db,
    database: sled::Db,
    reverse_chain: sled::Db,
    spent_tx: sled::Db,
}

impl Blockchain {
    pub fn new(data_dir: &std::path::Path) -> Blockchain {
        let mut blockchain_dir = std::path::PathBuf::new();
        blockchain_dir.push(data_dir);
        blockchain_dir.push("blockchain");
        let database = sled::Db::start_default(blockchain_dir).unwrap();

        let mut rev_dir = std::path::PathBuf::new();
        rev_dir.push(data_dir);
        rev_dir.push("reverse_chain");
        let reverse_chain = sled::Db::start_default(rev_dir).unwrap();

        let mut spent_tx_dir = std::path::PathBuf::new();
        spent_tx_dir.push(data_dir);
        spent_tx_dir.push("spent_tx");
        let spent_tx = sled::Db::start_default(spent_tx_dir).unwrap();

        let mut stats_dir = std::path::PathBuf::new();
        stats_dir.push(data_dir);
        stats_dir.push("stats");
        let stats = sled::Db::start_default(stats_dir).unwrap();

        Blockchain {
            stats,
            database,
            reverse_chain,
            spent_tx,
        }
    }

    pub fn block_after(&self, hash: &Sha256Result) -> Result<Option<Sha256Result>, Error> {
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.reverse_chain.get(&hash.serialize())? {
                Some(b) => (*b).to_owned(),
                None => return Ok(None),
            },
        ));
        Sha256Result::deserialize(&mut de)
            .map(|h| Some(h))
            .map_err(|e| Error::ParseError(e))
    }

    pub fn best_block_hash(&self) -> Result<Sha256Result, Error> {
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.stats.get("best_block")? {
                Some(b) => (*b).to_owned(),
                None => return Err(Error::NotFound),
            },
        ));
        Sha256Result::deserialize(&mut de).map_err(|e| Error::ParseError(e))
    }

    pub fn exists(&self, hash: &ensicoin_serializer::Sha256Result) -> Result<bool, Error> {
        self.database
            .contains_key(hash)
            .map_err(|e| Error::DatabaseError(e))
    }

    pub fn find_last_common_block(
        &self,
        block_locator: &Vec<ensicoin_serializer::Sha256Result>,
    ) -> Result<Option<Sha256Result>, Error> {
        for hash in block_locator.iter() {
            if self.exists(hash)? {
                return Ok(Some(hash.clone()));
            }
        }
        Ok(None)
    }

    pub fn generate_inv(
        &self,
        get_blocks: &ensicoin_messages::message::GetBlocks,
    ) -> Result<ensicoin_messages::message::Inv, Error> {
        let mut inv = ensicoin_messages::message::Inv {
            inventory: Vec::new(),
        };
        let last_common_block = match self.find_last_common_block(&get_blocks.block_locator)? {
            Some(h) => h,
            None => return Ok(inv),
        };
        let uptil = if self.exists(&get_blocks.stop_hash)? {
            get_blocks.stop_hash.clone()
        } else if get_blocks.stop_hash.iter().all(|b| *b == 0) {
            self.best_block_hash()?
        } else {
            return Ok(inv);
        };
        let mut hash = match self.block_after(&last_common_block)? {
            Some(h) => h,
            None => return Ok(inv),
        };
        while hash != uptil {
            inv.inventory.push(ensicoin_messages::message::InvVect {
                data_type: ensicoin_messages::message::ResourceType::Block,
                hash: hash,
            });
            hash = match self.block_after(&inv.inventory.last().unwrap().hash)? {
                Some(h) => h,
                None => return Ok(inv),
            }
        }
        Ok(inv)
    }

    pub fn add_block(&mut self, block: &Block) -> Result<(), Error> {
        let raw_block = block.serialize().to_vec();
        let hash = block.double_hash();
        //let utxo = block.utxo().serialize().to_vec();
        let spent_tx = Vec::new();
        self.database.set(hash, raw_block.clone())?;
        self.reverse_chain.set(block.header.prev_block, raw_block)?;
        self.spent_tx.set(hash, spent_tx)?;
        Ok(())
    }
}

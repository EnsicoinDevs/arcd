use crate::{
    data::{linkedblock::LinkedBlock, PairedUtxo},
    Error,
};
use ensicoin_messages::resource::{Block, Outpoint, Transaction};
use ensicoin_serializer::{hash_to_string, Deserialize, Serialize, Sha256Result};
use num_bigint::BigUint;

pub struct Blockchain {
    stats: sled::Db,
    database: sled::Db,
    reverse_chain: sled::Db,
    spent_tx: sled::Db,
    past_block: sled::Db,
    work: sled::Db,
}

pub enum NewAddition {
    Fork,
    BestBlock,
    Nothing,
}

pub struct PopContext {
    pub utxo_to_remove: Vec<Outpoint>,
    pub utxo_to_restore: Vec<PairedUtxo>,
    pub txs_to_restore: Vec<Transaction>,
}

impl Blockchain {
    #[cfg(not(feature = "grpc"))]
    pub fn read(&self) -> Result<&Self, Error> {
        Ok(&self)
    }
    #[cfg(not(feature = "grpc"))]
    pub fn write(&mut self) -> Result<&mut Self, Error> {
        Ok(self)
    }

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

        let mut past_dir = std::path::PathBuf::new();
        past_dir.push(data_dir);
        past_dir.push("past_block");
        let past_block = sled::Db::start_default(past_dir).unwrap();

        let mut work_dir = std::path::PathBuf::new();
        work_dir.push(data_dir);
        work_dir.push("work");
        let work = sled::Db::start_default(work_dir).unwrap();

        Blockchain {
            stats,
            database,
            reverse_chain,
            spent_tx,
            past_block,
            work,
        }
    }

    pub fn block_after(&self, hash: &Sha256Result) -> Result<Option<Sha256Result>, Error> {
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.reverse_chain.get(&hash)? {
                Some(b) => (*b).to_owned(),
                None => return Ok(None),
            },
        ));
        Sha256Result::deserialize(&mut de)
            .map(Some)
            .map_err(Error::ParseError)
    }

    fn get_work(&self, hash: &Sha256Result) -> Result<BigUint, Error> {
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.work.get(&hash)? {
                Some(b) => (*b).to_owned(),
                None => return Err(Error::NotFound(format!("work {}", hash_to_string(&hash)))),
            },
        ));
        Vec::deserialize(&mut de)
            .map(|b| BigUint::from_bytes_be(&b))
            .map_err(Error::ParseError)
    }

    pub fn get_block(&self, hash: &Sha256Result) -> Result<Option<Block>, Error> {
        debug!("Getting block from db");
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.database.get(&hash)? {
                Some(b) => (*b).to_owned(),
                None => return Ok(None),
            },
        ));
        Block::deserialize(&mut de).map(Some).map_err(|e| {
            warn!("Error parsing block {} from db", hash_to_string(hash));
            Error::ParseError(e)
        })
    }

    pub fn block_2016_before(&self, hash: &Sha256Result) -> Result<Sha256Result, Error> {
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.past_block.get(&hash)? {
                Some(b) => (*b).to_owned(),
                None => {
                    return Err(Error::NotFound(format!(
                        "2016 before {}",
                        hash_to_string(hash)
                    )))
                }
            },
        ));
        Sha256Result::deserialize(&mut de).map_err(Error::ParseError)
    }

    pub fn best_block_hash(&self) -> Result<Sha256Result, Error> {
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.stats.get("best_block")? {
                Some(b) => (*b).to_owned(),
                None => return Err(Error::NotFound("best_block".to_string())),
            },
        ));
        Sha256Result::deserialize(&mut de).map_err(Error::ParseError)
    }

    pub fn genesis_hash(&self) -> Result<Sha256Result, Error> {
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.stats.get("genesis_block")? {
                Some(b) => (*b).to_owned(),
                None => return Err(Error::NotFound("genesis_block".to_string())),
            },
        ));
        Sha256Result::deserialize(&mut de).map_err(Error::ParseError)
    }

    fn set_best_block(&mut self, hash: Sha256Result) -> Result<(), Error> {
        self.stats.set("best_block", hash.serialize().to_vec())?;
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.stats.get("10_last")? {
                Some(b) => (*b).to_owned(),
                None => return Err(Error::NotFound("10_last".to_string())),
            },
        ));
        let mut blocks: Vec<Sha256Result> = Vec::deserialize(&mut de)?;
        blocks.push(hash);
        if blocks.len() > 10 {
            blocks = blocks.split_off(1);
        }
        self.stats.set("10_last", blocks.serialize().to_vec())?;
        Ok(())
    }

    fn unset_best_block(&mut self) -> Result<(), Error> {
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.stats.get("10_last")? {
                Some(b) => (*b).to_owned(),
                None => return Err(Error::NotFound("10_last".to_string())),
            },
        ));
        let mut blocks: Vec<Sha256Result> = Vec::deserialize(&mut de)?;
        blocks.pop();
        let last_block = self.get_block(&blocks[0])?.unwrap().header;
        if last_block.prev_block != Sha256Result::from([0; 32]) {
            let mut temp = blocks;
            blocks = vec![last_block.prev_block];
            blocks.append(&mut temp);
        }
        self.stats.set("10_last", blocks.serialize().to_vec())?;
        Ok(())
    }

    pub fn generate_get_blocks(&self) -> Result<ensicoin_messages::message::GetBlocks, Error> {
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.stats.get("10_last")? {
                Some(b) => (*b).to_owned(),
                None => return Err(Error::NotFound("10_last".to_string())),
            },
        ));
        let mut blocks: Vec<Sha256Result> = Vec::deserialize(&mut de)?;
        blocks.reverse();
        if blocks.len() > 10 {
            blocks.push(self.genesis_hash()?);
        };
        Ok(ensicoin_messages::message::GetBlocks {
            stop_hash: Sha256Result::from([0 as u8; 32]),
            block_locator: blocks,
        })
    }

    pub fn exists(&self, hash: &ensicoin_serializer::Sha256Result) -> Result<bool, Error> {
        self.database
            .contains_key(hash)
            .map_err(Error::DatabaseError)
    }

    pub fn get_unknown_blocks(
        &self,
        inv: Vec<ensicoin_messages::message::InvVect>,
    ) -> Result<
        (
            Vec<ensicoin_messages::message::InvVect>,
            Vec<ensicoin_messages::message::InvVect>,
        ),
        Error,
    > {
        let mut unknown = Vec::new();
        let mut remaining = Vec::new();
        for inv_vect in inv {
            match inv_vect.data_type {
                ensicoin_messages::message::ResourceType::Block => {
                    if !self.exists(&inv_vect.hash)? {
                        unknown.push(inv_vect);
                    }
                }
                _ => remaining.push(inv_vect),
            }
        }
        Ok((unknown, remaining))
    }

    pub fn find_last_common_block(
        &self,
        block_locator: &[ensicoin_serializer::Sha256Result],
    ) -> Result<Option<Sha256Result>, Error> {
        for hash in block_locator.iter() {
            if self.exists(hash)? {
                return Ok(Some(*hash));
            }
        }
        Ok(None)
    }

    pub fn generate_inv(
        &self,
        get_blocks: &ensicoin_messages::message::GetBlocks,
    ) -> Result<ensicoin_messages::message::Inv, Error> {
        let inv = ensicoin_messages::message::Inv {
            inventory: Vec::new(),
        };
        let last_common_block = match self.find_last_common_block(&get_blocks.block_locator)? {
            Some(h) => h,
            None => return Ok(inv),
        };
        let uptil = if self.exists(&get_blocks.stop_hash)? {
            get_blocks.stop_hash
        } else if get_blocks.stop_hash.iter().all(|b| *b == 0) {
            self.best_block_hash()?
        } else {
            return Ok(inv);
        };
        let chain = self.chain_until(&uptil, &last_common_block)?;
        Ok(ensicoin_messages::message::Inv {
            inventory: chain
                .into_iter()
                .map(|hash| ensicoin_messages::message::InvVect {
                    data_type: ensicoin_messages::message::ResourceType::Block,
                    hash,
                })
                .collect(),
        })
    }

    pub fn find_common_hash(
        &self,
        hash1: Sha256Result,
        hash2: Sha256Result,
    ) -> Result<Option<Sha256Result>, Error> {
        if !self.exists(&hash1)? || !self.exists(&hash2)? {
            info!("One does not exist");
            return Ok(None);
        };
        let mut block1 = match self.get_block(&hash1)? {
            Some(b) => b,
            None => {
                info!("Orphan block");
                return Ok(None);
            }
        };
        let mut block2 = match self.get_block(&hash2)? {
            Some(b) => b,
            None => {
                info!("Orphan block");
                return Ok(None);
            }
        };
        if block2.header.height > block1.header.height {
            std::mem::swap(&mut block1, &mut block2);
        }

        if block1.header.height > block2.header.height {
            while block1.header.height != block2.header.height {
                block1 = match self.get_block(&block1.header.prev_block)? {
                    Some(b) => b,
                    None => {
                        info!("Orphan chain");
                        return Ok(None);
                    }
                }
            }
        };
        while block1.double_hash() != block2.double_hash() {
            block1 = match self.get_block(&block1.header.prev_block)? {
                Some(b) => b,
                None => {
                    info!("No merge point");
                    return Ok(None);
                }
            };
            block2 = match self.get_block(&block2.header.prev_block)? {
                Some(b) => b,
                None => {
                    info!("No merge point");
                    return Ok(None);
                }
            };
        }
        if block1.double_hash() == block2.double_hash() {
            Ok(Some(block1.double_hash()))
        } else {
            Ok(None)
        }
    }

    pub fn new_block(&mut self, block: LinkedBlock) -> Result<NewAddition, Error> {
        let hash = block.header.double_hash();
        let chain_work = self.get_work(&block.header.prev_block)? + block.work();
        let best_hash = self.best_block_hash()?;
        let best_work = self.get_work(&best_hash)?;
        Ok(
            if block.header.prev_block == best_hash || chain_work > best_work {
                if block.header.prev_block != best_hash {
                    self.database
                        .set(hash, block.into_block().serialize().to_vec())?;
                    self.work
                        .set(hash, chain_work.to_bytes_be().serialize().to_vec())?;
                    NewAddition::Fork
                } else {
                    self.add_block(block)?;
                    NewAddition::BestBlock
                }
            } else {
                self.database
                    .set(hash, block.into_block().serialize().to_vec())?;
                self.work
                    .set(hash, chain_work.to_bytes_be().serialize().to_vec())?;
                NewAddition::Nothing
            },
        )
    }

    pub fn add_chain(&mut self, blocks: Vec<LinkedBlock>) -> Result<(), Error> {
        for b in blocks {
            self.add_block(b)?;
        }
        Ok(())
    }

    fn add_block(&mut self, block: LinkedBlock) -> Result<(), Error> {
        debug!(
            "Adding block {} to blockchain",
            hash_to_string(&block.header.double_hash())
        );
        let chain_work = self.get_work(&block.header.prev_block)? + block.work();
        let spent_utxo = block.spent_utxo().serialize().to_vec();
        let block = block.into_block();
        let raw_block = block.serialize().to_vec();
        let hash = block.header.double_hash();
        if block.header.height == 2015 {
            let genesis_hash = self.genesis_hash()?;
            self.past_block
                .set(hash, genesis_hash.serialize().to_vec())?;
        } else if block.header.height >= 2016 {
            let past_of_previous = self.block_2016_before(&block.header.prev_block)?;
            let next = self.block_after(&past_of_previous)?.unwrap();
            self.past_block.set(hash, next.serialize().to_vec())?;
        };
        self.work
            .set(hash, chain_work.to_bytes_be().serialize().to_vec())?;
        self.database.set(hash, raw_block.clone())?;
        self.reverse_chain
            .set(block.header.prev_block, hash.serialize().to_vec())?;
        self.spent_tx.set(hash, spent_utxo)?;
        self.set_best_block(hash)?;
        Ok(())
    }

    pub fn pop_until(&mut self, hash: &Sha256Result) -> Result<PopContext, Error> {
        let until_block = match self.get_block(hash)? {
            Some(b) => b,
            None => return Err(Error::NotFound(format!("block {}", hash_to_string(&hash)))),
        };
        let until_height = until_block.header.height;
        let mut utxo_to_restore = Vec::new();
        let mut utxo_to_remove = Vec::new();
        let mut txs_to_restore = Vec::new();
        while self.best_block_hash()? != *hash {
            let mut pop_context = self.pop_best_block()?;
            utxo_to_restore.extend(
                pop_context
                    .utxo_to_restore
                    .into_iter()
                    .filter(|pairedutxo| pairedutxo.data.block_height <= until_height),
            );
            utxo_to_remove.append(&mut pop_context.utxo_to_remove);
            txs_to_restore.append(&mut pop_context.txs_to_restore);
        }
        Ok(PopContext {
            txs_to_restore,
            utxo_to_remove,
            utxo_to_restore,
        })
    }

    // returns (until ; from]
    pub fn chain_until(
        &self,
        from: &Sha256Result,
        until: &Sha256Result,
    ) -> Result<Vec<Sha256Result>, Error> {
        let mut blocks = Vec::new();
        let mut hash = *from;
        while hash != *until {
            let block = match self.get_block(&hash)? {
                Some(b) => b,
                None => {
                    return Err(Error::NotFound(format!(
                        "prev of {}",
                        hash_to_string(&hash)
                    )))
                }
            };
            blocks.push(hash);
            hash = block.header.prev_block;
        }
        blocks.reverse();
        Ok(blocks)
    }

    pub fn chain_to_blocks(&self, chain: Vec<Sha256Result>) -> Result<Vec<Block>, Error> {
        let mut new_chain = Vec::with_capacity(chain.len());
        for hash in chain {
            new_chain.push(match self.get_block(&hash)? {
                Some(b) => b,
                None => return Err(Error::NotFound(format!("block {}", hash_to_string(&hash)))),
            })
        }
        Ok(new_chain)
    }

    pub fn pop_best_block(&mut self) -> Result<PopContext, Error> {
        let best_block = self.best_block_hash()?;
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.spent_tx.get(&best_block)? {
                Some(b) => (*b).to_owned(),
                None => {
                    return Err(Error::NotFound(format!(
                        "spent tx {}",
                        hash_to_string(&best_block)
                    )))
                }
            },
        ));
        let best_block = self.get_block(&best_block)?.unwrap();
        self.reverse_chain.del(&best_block.header.prev_block)?;
        self.unset_best_block()?;
        self.stats.set(
            "best_block",
            best_block.header.prev_block.serialize().to_vec(),
        )?;
        let utxo_to_restore = Vec::deserialize(&mut de)?;
        let mut utxo_to_remove = Vec::new();
        let mut txs_to_restore = Vec::new();
        for tx in best_block.txs {
            let tx_hash = tx.double_hash();
            for i in 0..tx.outputs.len() {
                utxo_to_remove.push(Outpoint {
                    hash: tx_hash,
                    index: (i as u32),
                })
            }
            txs_to_restore.push(tx)
        }
        Ok(PopContext {
            txs_to_restore,
            utxo_to_remove,
            utxo_to_restore,
        })
    }

    pub fn get_target_next_block(&self, timestamp: u64) -> Result<BigUint, Error> {
        use crate::constants::TIME_BEETWEEN_BLOCKS;

        let best_block = self.get_block(&self.best_block_hash()?)?.unwrap();
        let mut ancestor = self.get_block(&self.genesis_hash()?)?.unwrap();
        if best_block.header.height >= 2015 {
            ancestor = self
                .get_block(
                    &self
                        .block_after(&self.block_2016_before(&self.best_block_hash()?)?)?
                        .unwrap(),
                )?
                .unwrap();

            let old_target = BigUint::from_bytes_be(&best_block.header.target);
            let mut time_diff = timestamp - ancestor.header.timestamp;
            if time_diff > 4 * TIME_BEETWEEN_BLOCKS {
                time_diff = 4 * TIME_BEETWEEN_BLOCKS
            } else if time_diff < TIME_BEETWEEN_BLOCKS / 4 {
                time_diff = TIME_BEETWEEN_BLOCKS / 4
            };
            Ok(std::cmp::min(
                (old_target * BigUint::from(time_diff)) / BigUint::from(TIME_BEETWEEN_BLOCKS),
                (BigUint::from(1 as u64) << 256) - (1 as u64),
            ))
        } else {
            Ok(BigUint::from_bytes_be(&ancestor.header.target))
        }
    }

    pub fn get_data(
        &self,
        inv: Vec<ensicoin_messages::message::InvVect>,
    ) -> Result<
        (
            Vec<ensicoin_messages::resource::Block>,
            Vec<ensicoin_messages::message::InvVect>,
        ),
        Error,
    > {
        let mut remaining = Vec::new();
        let mut blocks = Vec::new();
        for inv_vect in inv {
            match inv_vect.data_type {
                ensicoin_messages::message::ResourceType::Block => {
                    if let Some(b) = self.get_block(&inv_vect.hash)? {
                        blocks.push(b);
                    }
                }
                _ => remaining.push(inv_vect),
            }
        }
        Ok((blocks, remaining))
    }
}

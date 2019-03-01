use super::tx::Outpoint;
use super::tx::Transaction;
use crate::manager::UtxoData;
use std::collections::HashMap;

pub enum Dependency {
    Utxo(UtxoData),
    Pending,
    Mempool,
}

pub struct LinkedTransaction {
    transaction: Transaction,
    input_count: usize,
    dependencies: HashMap<Outpoint, Dependency>,
    dep_count: usize,
    mempool_count: usize,
}

impl LinkedTransaction {
    pub fn new(tx: Transaction) -> LinkedTransaction {
        LinkedTransaction {
            transaction: tx,
            input_count: 0,
            dependencies: HashMap::new(),
            dep_count: 0,
            mempool_count: 0,
        }
    }

    pub fn add_dependency(&mut self, outpoint: Outpoint, dep: Dependency) {
        if let Dependency::Mempool = dep {
            self.mempool_count += 1;
        };
        if let None = self.dependencies.insert(outpoint, dep) {
            self.dep_count += 1;
        };
    }

    pub fn toggle_mempool(&mut self, outpoint: Outpoint) {
        match self.dependencies.get(&outpoint) {
            Some(Dependency::Pending) => {
                self.dependencies.insert(outpoint, Dependency::Mempool);
                self.mempool_count += 1;
            }
            Some(Dependency::Mempool) => {
                self.dependencies.insert(outpoint, Dependency::Pending);
                self.mempool_count -= 1;
            }
            _ => (),
        };
    }

    pub fn is_complete(&self) -> bool {
        self.dep_count == self.input_count
    }

    pub fn is_publishable(&self) -> bool {
        self.mempool_count == 0 && self.is_complete()
    }
}

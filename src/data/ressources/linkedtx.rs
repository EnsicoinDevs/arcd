use super::tx::Outpoint;
use super::tx::Transaction;
use crate::manager::UtxoData;
use std::collections::HashMap;

pub struct Dependency {
    pub dep_type: DependencyType,
    pub data: UtxoData,
}

pub enum DependencyType {
    Block,
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
        if let DependencyType::Mempool = dep.dep_type {
            self.mempool_count += 1;
        };
        if let None = self.dependencies.insert(outpoint, dep) {
            self.dep_count += 1;
        };
    }

    pub fn toggle_mempool(&mut self, outpoint: Outpoint) {
        if let Some(d) = self.dependencies.get_mut(&outpoint) {
            match d.dep_type {
                DependencyType::Mempool => {
                    d.dep_type = DependencyType::Pending;
                    self.mempool_count -= 1;
                }
                DependencyType::Pending => {
                    d.dep_type = DependencyType::Mempool;
                    self.mempool_count += 1;
                }
                _ => (),
            }
        }
    }

    pub fn is_complete(&self) -> bool {
        self.dep_count == self.input_count
    }

    pub fn is_publishable(&self) -> bool {
        self.mempool_count == 0 && self.is_complete()
    }
}

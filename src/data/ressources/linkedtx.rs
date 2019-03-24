use super::tx::Outpoint;
use super::tx::Transaction;
use crate::constants::Sha256Result;
use crate::manager::UtxoData;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::Deref;

#[derive(PartialEq, Eq)]
pub struct Dependency {
    pub dep_type: DependencyType,
    pub data: UtxoData,
}

#[derive(PartialEq, Eq)]
pub enum DependencyType {
    Block,
    Pending,
    Mempool,
}

pub struct LinkedTransaction {
    transaction: Transaction,
    input_count: usize,
    dependencies: HashMap<Outpoint, Dependency>,
    unknown_parent: HashSet<Outpoint>,
    dep_count: usize,
    mempool_count: usize,
}

impl PartialEq for LinkedTransaction {
    fn eq(&self, other: &LinkedTransaction) -> bool {
        self.transaction == other.transaction
    }
}

impl Eq for LinkedTransaction {}

impl Deref for LinkedTransaction {
    type Target = Transaction;

    fn deref(&self) -> &Transaction {
        &self.transaction
    }
}

impl LinkedTransaction {
    pub fn new(tx: Transaction) -> LinkedTransaction {
        let mut ltx = LinkedTransaction {
            transaction: tx,
            input_count: 0,
            dependencies: HashMap::new(),
            unknown_parent: HashSet::new(),
            dep_count: 0,
            mempool_count: 0,
        };
        ltx.init_parents();
        ltx
    }

    fn init_parents(&mut self) {
        for input in self.transaction.get_inputs() {
            self.unknown_parent.insert(input.previous_output.clone());
        }
    }

    pub fn unknown(&self) -> &HashSet<Outpoint> {
        &self.unknown_parent
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

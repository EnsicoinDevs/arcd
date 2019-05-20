use crate::data::{validation::SanityCheck, PairedUtxo, UtxoData};
use ensicoin_messages::resource::{Outpoint, Transaction};
use ensicoin_serializer::Serialize;
use std::collections::{HashMap, HashSet};

use sha2::Digest;

#[derive(PartialEq, Eq, Clone)]
pub struct Dependency {
    pub dep_type: DependencyType,
    pub data: UtxoData,
}

#[derive(PartialEq, Eq, Clone)]
pub enum DependencyType {
    Block,
    Mempool,
}

#[derive(Clone)]
pub struct LinkedTransaction {
    pub transaction: Transaction,
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
        for input in self.transaction.inputs.iter() {
            self.unknown_parent.insert(input.previous_output.clone());
        }
    }

    pub fn unknown(&self) -> &HashSet<Outpoint> {
        &self.unknown_parent
    }

    pub fn get_dependent_utxo(&self) -> Vec<PairedUtxo> {
        let mut utxos = Vec::new();
        for (out, dep) in self.dependencies.iter() {
            utxos.push(PairedUtxo {
                data: dep.data.clone(),
                outpoint: out.clone(),
            })
        }
        utxos
    }

    pub fn add_dependency(&mut self, outpoint: Outpoint, dep: Dependency) {
        self.unknown_parent.remove(&outpoint);
        if let DependencyType::Mempool = dep.dep_type {
            self.mempool_count += 1;
        };
        if let None = self.dependencies.insert(outpoint, dep) {
            self.dep_count += 1;
        };
    }

    pub fn is_complete(&self) -> bool {
        self.dep_count == self.input_count
    }

    pub fn is_valid(&self) -> Result<bool, ()> {
        if !self.transaction.sanity_check() {
            return Ok(false);
        };

        let mut hasher_outpoints = sha2::Sha256::default();
        for input in &self.transaction.inputs {
            hasher_outpoints.input(input.previous_output.serialize());
        }
        let hash = hasher_outpoints.result();
        let mut hasher_outpoints = sha2::Sha256::default();
        hasher_outpoints.input(hash);

        let hash_outpoints = hasher_outpoints.result();

        let mut hasher_outputs = sha2::Sha256::default();
        for output in &self.transaction.outputs {
            hasher_outputs.input(output.serialize());
        }
        let hash_outputs = hasher_outputs.result();

        for input in &self.transaction.inputs {
            let mut script = input.script.clone();
            script.append(&mut match self.dependencies.get(&input.previous_output) {
                Some(dep) => dep.data.script.clone(),
                _ => return Err(()),
            });
            let mut hasher = sha2::Sha256::default();
            hasher.input(self.transaction.version.serialize());
            hasher.input(self.transaction.flags.serialize());
            hasher.input(&hash_outpoints);
            hasher.input(input.previous_output.serialize());
            hasher.input(match self.dependencies.get(&input.previous_output) {
                Some(dep) => dep.data.value.serialize(),
                _ => return Err(()),
            });
            hasher.input(&hash_outputs);

            if !crate::data::script_vm::execute_script(script, hasher.result()) {
                return Ok(false);
            }
        }

        let mut output_sum = 0;
        for output in &self.transaction.outputs {
            output_sum += output.value;
        }
        let mut input_sum = 0;
        for (_, input) in &self.dependencies {
            input_sum += input.data.value
        }
        Ok(input_sum < output_sum)
    }
}

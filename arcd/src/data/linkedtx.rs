use crate::data::{validation::SanityCheck, PairedUtxo, UtxoData};
use ensicoin_messages::resource::{Outpoint, Transaction, tx::{fn_outpoint, fn_tx_input, fn_tx_output}, script::fn_script};
use std::collections::{HashMap, HashSet};
use cookie_factory::{bytes::{be_u32, be_u64}};
use ensicoin_serializer::{serializer::{fn_str, fn_list}};

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
        if self.dependencies.insert(outpoint, dep).is_none() {
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
            hasher_outpoints.input(ensicoin_messages::as_bytes(fn_outpoint(&input.previous_output)));
        }
        let simple_outpoint_hash = hasher_outpoints.result();

        let mut hasher_outpoints = sha2::Sha256::default();
        hasher_outpoints.input(simple_outpoint_hash);
        let hash_outpoints = hasher_outpoints.result();

        let mut hasher_outputs = sha2::Sha256::default();
        for output in &self.transaction.outputs {
            hasher_outputs.input(ensicoin_messages::as_bytes(fn_tx_output(output)));
        }
        let hash_outputs = hasher_outputs.result();

        for input in &self.transaction.inputs {
            let mut script = input.script.clone();
            script.concat(match self.dependencies.get(&input.previous_output) {
                Some(dep) => dep.data.script.clone(),
                _ => return Err(()),
            });
            let mut hasher = sha2::Sha256::default();
            hasher.input(ensicoin_messages::as_bytes(be_u32(self.transaction.version)));
            hasher.input(ensicoin_messages::as_bytes(fn_list(self.transaction.flags.len() as u64, self.transaction.flags.iter().map(fn_str))));
            hasher.input(&hash_outpoints);
            hasher.input(ensicoin_messages::as_bytes(fn_outpoint(&input.previous_output)));
            hasher.input(match self.dependencies.get(&input.previous_output) {
                Some(dep) => ensicoin_messages::as_bytes(be_u64(dep.data.value)),
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
        for input in self.dependencies.values() {
            input_sum += input.data.value
        }
        Ok(input_sum < output_sum)
    }
}

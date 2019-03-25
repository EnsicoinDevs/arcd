use crate::constants::Sha256Result;
use crate::data::ressources::{LinkedTransaction, Outpoint, Dependency, DependencyType};
use std::collections::HashMap;

type Dep = (Sha256Result, Outpoint);

pub struct Mempool {
    pool: HashMap<Sha256Result, LinkedTransaction>,
    orphan: HashMap<Sha256Result, LinkedTransaction>,

    dependencies: HashMap<Sha256Result, Vec<Dep>>,
}

impl Mempool {
    pub fn new() -> Mempool {
        Mempool {
            pool: HashMap::new(),
            orphan: HashMap::new(),

            dependencies: HashMap::new(),
        }
    }

    fn added_parent_to_pool(&mut self, hash_tx: Sha256Result) {
        if let Some(dependencies) = self.dependencies.get(&hash_tx).cloned() {
            for (orphan_hash, outpoint) in dependencies {
                self.orphan.get_mut(&orphan_hash).unwrap().add_dependency(outpoint.clone(), Dependency {
                    dep_type: DependencyType::Mempool,
                    data: self.pool.get(&hash_tx).unwrap().transaction.get_data(outpoint.index as usize, false, 0)
                });
                if self.orphan.get(&orphan_hash).unwrap().is_complete() {
                    self.pool.insert(orphan_hash.clone(), self.orphan.remove(&orphan_hash).unwrap());
                    self.added_parent_to_pool(orphan_hash.clone());
                }
            } 
        }
    }

    fn link(&mut self, linked_tx: &mut LinkedTransaction) {
        for parent in linked_tx.unknown().clone() {
            match self.pool.get(&parent.hash) {
                Some(parent_tx) => {
                    let parent_data =
                        parent_tx
                            .transaction
                            .get_data(parent.index as usize, false, 0);
                    linked_tx.add_dependency(
                        parent,
                        crate::data::ressources::Dependency {
                            dep_type: crate::data::ressources::DependencyType::Mempool,
                            data: parent_data,
                        },
                    );
                }
                None => {
                    if let None = self.dependencies.get(&parent.hash) {
                        self.dependencies.insert(parent.hash.clone(), Vec::new());
                    };
                    self.dependencies.get_mut(&parent.hash).unwrap().push((
                        Sha256Result::from_slice(&linked_tx.transaction.double_hash()).clone(),
                        parent,
                    ));
                }
            }
        }
    }

    pub fn insert(&mut self, mut linked_tx: LinkedTransaction) {
        self.link(&mut linked_tx);
        let hash = linked_tx.transaction.double_hash();
        if linked_tx.is_complete() {
            self.pool.insert(hash, linked_tx);
            self.added_parent_to_pool(hash);
        } else {
            self.orphan.insert(hash, linked_tx);
        }
    }
}

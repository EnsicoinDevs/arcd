use crate::constants::Sha256Result;
use crate::data::ressources::LinkedTransaction;
use std::collections::HashMap;

pub struct Mempool {
    pool: HashMap<Sha256Result, LinkedTransaction>,
    orphan: HashMap<Sha256Result, LinkedTransaction>,

    dependency: HashMap<Sha256Result, Vec<Sha256Result>>,
}

impl Mempool {
    pub fn new() -> Mempool {
        Mempool {
            pool: HashMap::new(),
            orphan: HashMap::new(),

            dependency: HashMap::new(),
        }
    }

    fn link(&mut self, linked_tx: &mut LinkedTransaction) {
        for parent in linked_tx.unknown().clone() {
            let parent_key = Sha256Result::from_slice(&parent.hash.value);
            match self.pool.get(parent_key) {
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
                    if let None = self.dependency.get(parent_key) {
                        self.dependency.insert(parent_key.clone(), Vec::new());
                    };
                    self.dependency.get_mut(parent_key).unwrap().push(
                        Sha256Result::from_slice(&linked_tx.transaction.double_hash()).clone(),
                    );
                }
            }
        }
    }

    pub fn insert(&mut self, mut linked_tx: LinkedTransaction) {
        self.link(&mut linked_tx);
        let hash = linked_tx.transaction.double_hash();
        if linked_tx.is_complete() {
            self.pool.insert(hash, linked_tx);
        } else {
            self.orphan.insert(hash, linked_tx);
        }
    }
}

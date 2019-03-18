use crate::data::ressources::LinkedTransaction;
use std::collections::HashSet;

pub struct Mempool {
    pool: HashSet<LinkedTransaction>,
}

impl Mempool {
    pub fn new() -> Mempool {
        Mempool {
            pool: HashSet::new(),
        }
    }
}

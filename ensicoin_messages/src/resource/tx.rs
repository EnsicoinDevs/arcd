use ensicoin_serializer::{
    serializer::{fn_list, fn_str},
    types::Sha256Result,
    Deserialize,
};
use sha2::Digest;

use super::script::{fn_script, Script};
use cookie_factory::{
    bytes::{be_u32, be_u64},
    combinator::slice,
    sequence::tuple,
    SerializeFn,
};
use std::io::Write;

#[derive(Hash, Eq, PartialEq, Clone, Deserialize, Debug)]
pub struct Outpoint {
    pub hash: Sha256Result,
    pub index: u32,
}

pub fn fn_outpoint<'c, 'a: 'c, W: Write + 'c>(outpoint: &'a Outpoint) -> impl SerializeFn<W> + 'c {
    tuple((slice(outpoint.hash), be_u32(outpoint.index)))
}

#[derive(Hash, PartialEq, Eq, Deserialize, Clone, Debug)]
pub struct TransactionInput {
    pub previous_output: Outpoint,
    pub script: Script,
}

pub fn fn_tx_input<'c, 'a: 'c, W: Write + 'c>(
    tx_in: &'a TransactionInput,
) -> impl SerializeFn<W> + 'c {
    tuple((
        fn_outpoint(&tx_in.previous_output),
        fn_script(&tx_in.script),
    ))
}

#[derive(Hash, PartialEq, Eq, Deserialize, Clone, Debug)]
pub struct TransactionOutput {
    pub value: u64,
    pub script: Script,
}
pub fn fn_tx_output<'c, 'a: 'c, W: Write + 'c>(
    tx_out: &'a TransactionOutput,
) -> impl SerializeFn<W> + 'c {
    tuple((be_u64(tx_out.value), fn_script(&tx_out.script)))
}

#[derive(Hash, PartialEq, Eq, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub version: u32,
    pub flags: Vec<String>,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
}

pub fn fn_tx<'c, 'a: 'c, W: Write + 'c>(tx: &'a Transaction) -> impl SerializeFn<W> + 'c {
    tuple((
        be_u32(tx.version),
        fn_list(tx.flags.len() as u64, tx.flags.iter().map(fn_str)),
        fn_list(tx.inputs.len() as u64, tx.inputs.iter().map(fn_tx_input)),
        fn_list(tx.outputs.len() as u64, tx.outputs.iter().map(fn_tx_output)),
    ))
}

impl Transaction {
    pub fn serialize(&self) -> Vec<u8> {
        crate::as_bytes(fn_tx(self))
    }
    pub fn double_hash(&self) -> Sha256Result {
        let bytes = self.serialize();
        let mut hasher = sha2::Sha256::default();
        hasher.input(bytes);
        let first = hasher.result();
        hasher = sha2::Sha256::default();
        hasher.input(first);
        hasher.result()
    }

    pub fn shash(&self, i: usize, referenced_value: u64) -> Sha256Result {
        let mut hasher_outpoints = sha2::Sha256::default();
        for input in &self.inputs {
            hasher_outpoints.input(crate::as_bytes(fn_outpoint(&input.previous_output)));
        }
        let outpoint_simple_hash = hasher_outpoints.result();
        let mut hasher = sha2::Sha256::default();
        hasher.input(outpoint_simple_hash);
        let outpoint_hash = hasher.result();

        let mut hasher = sha2::Sha256::default();
        for output in &self.outputs {
            hasher.input(crate::as_bytes(fn_tx_output(&output)));
        }
        let hash_outputs = hasher.result();

        let mut hasher = sha2::Sha256::default();
        hasher.input(crate::as_bytes(be_u32(self.version)));
        hasher.input(crate::as_bytes(fn_list(
            self.flags.len() as u64,
            self.flags.iter().map(fn_str),
        )));
        hasher.input(outpoint_hash);
        hasher.input(crate::as_bytes(fn_outpoint(
            &self.inputs[i].previous_output,
        )));
        hasher.input(crate::as_bytes(be_u64(referenced_value)));
        hasher.input(hash_outputs);

        let simple_hash = hasher.result();
        let mut hasher = sha2::Sha256::default();
        hasher.input(simple_hash);
        hasher.result()
    }
}

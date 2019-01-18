extern crate bincode_modified;
use bincode_modified::{deserialize, serialize};

fn main() {
    let target = vec!["whoami".to_string()];
    let encoded: Vec<u8> = serialize(&target).unwrap();
    println!("Encoded : {:?}", encoded);
    let decoded: Vec<String> = deserialize(&encoded[..]).unwrap();
    println!("Decoded : {:?}", decoded);
}

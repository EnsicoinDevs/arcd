extern crate bincode_modified;
use bincode_modified::{deserialize, serialize};

fn main() {
    let target: Option<String> = Some("whoami".to_string());
    let encoded: Vec<u8> = serialize(&target).unwrap();
    println!("Encoded : {:?}", encoded);
}

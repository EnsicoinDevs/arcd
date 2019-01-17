#[macro_use]
extern crate serde_derive;
extern crate bincode_modified;

use bincode_modified::{deserialize, serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Entity {
    x: f32,
    y: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct World(Vec<Entity>);

fn main() {
    let vec: Vec<u8> = vec![1, 2, 3, 4, 5];
    let encoded: Vec<u8> = serialize(&vec).unwrap();
    println!("Vector : {:?}", encoded);

    let encoded_string: Vec<u8> = serialize(&"whoami".to_string()).unwrap();
    println!("whoami is {:?}", encoded_string);
}

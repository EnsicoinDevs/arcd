extern crate ensicoin_serializer;
use ensicoin_serializer::serializer::Serialize;

fn main() {
    let target = vec!["whoami".to_string()];
    let encoded: Vec<u8> = target.serialize();
    println!("Encoded : {:?}", encoded);
    //let decoded: Vec<String> = deserialize(&encoded[..]).unwrap();
    //println!("Decoded : {:?}", decoded);
}

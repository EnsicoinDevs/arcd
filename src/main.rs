extern crate ensicoin_serializer;
use ensicoin_serializer::deserializer::Deserialize;
use ensicoin_serializer::deserializer::Deserializer;
use ensicoin_serializer::serializer::Serialize;

fn main() {
    let target = vec!["whoami".to_string(), "whatareyoudoing".to_string()];
    let encoded: Vec<u8> = target.serialize();
    println!("Encoded : {:?}", encoded);
    let mut decoder = Deserializer::new(encoded);
    let decoded: Vec<String> = Vec::deserialize(&mut decoder).unwrap();
    println!("Decoded : {:?}", decoded);
}

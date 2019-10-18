extern crate bytes;
extern crate ensicoin_serializer;

use ensicoin_serializer::Deserialize;
use ensicoin_serializer::Serialize;

#[macro_use]
extern crate ensicoin_serializer_derive;

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct SomeStruct {
    pub thing: u8,
    pub gen_some: Vec<u64>,
}

#[test]
fn both_works() {
    let s = SomeStruct {
        thing: 3,
        gen_some: vec![1, 9],
    };
    let raw = s.serialize();
    let mut de = ensicoin_serializer::Deserializer::new(raw.try_mut().unwrap());
    let new_s = SomeStruct::deserialize(&mut de);
    assert_eq!(new_s.unwrap(), s);
}

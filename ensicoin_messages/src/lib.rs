extern crate bytes;
pub extern crate ensicoin_serializer;

#[macro_use]
extern crate ensicoin_serializer_derive;

pub mod message;
pub mod resource;

use cookie_factory::{gen_simple, SerializeFn};

pub fn as_bytes<F: SerializeFn<Vec<u8>>>(f: F) -> Vec<u8> {
    gen_simple(f, Vec::new()).expect("write in vec")
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

extern crate bytes;
pub extern crate ensicoin_serializer;

#[macro_use]
extern crate ensicoin_serializer_derive;

pub mod message;
pub mod resource;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

use std::net::{IpAddr, SocketAddr};

//extern crate ensicoin_serializer;
use ensicoin_serializer::Serialize;

use std::time::{SystemTime, UNIX_EPOCH};

pub struct Address {
    pub timestamp: u64,
    pub address: SocketAddr,
}

impl Serialize for Address {
    fn serialize(&self) -> Vec<u8> {
        let mut v = Vec::new();
        v.append(&mut self.timestamp.serialize());
        v.append(&mut self.address.serialize());
        v
    }
}

impl Address {
    pub fn new() -> Address {
        Address {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            address: SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 4224),
        }
    }
}

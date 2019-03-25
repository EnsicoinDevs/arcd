use std::net::{IpAddr, SocketAddr};

use ensicoin_serializer::Result as DeserResult;
use ensicoin_serializer::{Deserialize, Deserializer, Serialize};

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
pub struct Address {
    pub timestamp: u64,
    pub address: SocketAddr,
}

impl Deserialize for Address {
    fn deserialize(de: &mut Deserializer) -> DeserResult<Address> {
        Ok(Address {
            timestamp: match u64::deserialize(de) {
                Ok(x) => x,
                Err(e) => {
                    return Err(ensicoin_serializer::Error::Message(format!(
                        "In Address reading timestamp: {}",
                        e
                    )));
                }
            },
            address: match SocketAddr::deserialize(de) {
                Ok(x) => x,
                Err(e) => {
                    return Err(ensicoin_serializer::Error::Message(format!(
                        "In Address reading address: {}",
                        e
                    )));
                }
            },
        })
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

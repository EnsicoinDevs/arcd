use crate::message::Address;
use ensicoin_serializer::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Whoami {
    pub version: u32,
    pub address: Address,
    pub services: Vec<String>,
}

impl Whoami {
    pub fn new(address: Address) -> Whoami {
        Whoami {
            version: 1,
            address,
            services: vec!["node".to_string()],
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct WhoamiAck {}

impl WhoamiAck {
    pub fn new() -> WhoamiAck {
        WhoamiAck {}
    }
}

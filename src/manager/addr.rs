use ensicoin_messages::message::{Addr, Address};
use ensicoin_serializer::{Deserialize, Serialize};

#[derive(Debug)]
pub enum AddressManagerError {
    MissingKey(String),
    ParseError(ensicoin_serializer::Error),
    DbError(sled::Error),
}

impl std::fmt::Display for AddressManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AddressManagerError::MissingKey(key) => write!(f, "Key {} is not in the database", key),
            AddressManagerError::DbError(e) => write!(f, "Error in database: {}", e),
            AddressManagerError::ParseError(e) => write!(f, "Error parsing data: {}", e),
        }
    }
}

impl std::error::Error for AddressManagerError {}

impl From<ensicoin_serializer::Error> for AddressManagerError {
    fn from(err: ensicoin_serializer::Error) -> Self {
        AddressManagerError::ParseError(err)
    }
}

impl From<sled::Error> for AddressManagerError {
    fn from(err: sled::Error) -> Self {
        AddressManagerError::DbError(err)
    }
}

#[derive(Serialize, Deserialize)]
struct Peer {
    pub addr: Address,
    pub orphan_count: u64,
    pub connection_error: u64,
}

pub struct AddressManager {
    address: Vec<Peer>,
    db: sled::Db,
}

impl AddressManager {
    pub fn new(data_dir: &std::path::Path) -> Result<Self, AddressManagerError> {
        let mut db_dir = std::path::PathBuf::new();
        db_dir.push(data_dir);
        db_dir.push("adress_manager");
        let db = sled::Db::start_default(db_dir)?;

        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match db.get("address")? {
                Some(b) => (*b).to_owned(),
                None => return Err(AddressManagerError::MissingKey("address".to_string())),
            },
        ));
        let address = Vec::deserialize(&mut de)?;
        Ok(AddressManager { address, db })
    }

    pub fn get_addr(&self) -> Addr {
        let addresses = self.address.iter().map(|p| p.addr.clone()).collect();
        Addr { addresses }
    }

    pub fn save(&self) -> Result<(), AddressManagerError> {
        self.db.set("address", self.address.serialize().to_vec())?;
        Ok(())
    }
}

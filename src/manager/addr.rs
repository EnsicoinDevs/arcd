use crate::data::intern_messages::{Peer, Source};
use ensicoin_messages::message::{Addr, Address};
use ensicoin_serializer::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub enum AddressManagerError {
    //MissingKey(String),
    ParseError(ensicoin_serializer::Error),
    DbError(sled::Error),
}

impl std::fmt::Display for AddressManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            //AddressManagerError::MissingKey(key) => write!(f, "Key {} is not in the database", key),
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
struct PeerData {
    pub timestamp: u64,
}

pub struct AddressManager {
    db: sled::Db,
}

impl AddressManager {
    pub fn set_bots(&mut self, bots: Vec<String>) {
        for bot in bots {
            let bot = bot.trim_start_matches(&format!("{}_", crate::constants::MAGIC));

            let addr: std::net::SocketAddr = match bot.parse() {
                Ok(addr) => addr,
                Err(_) => {
                    debug!("Bot has incorrect address: {}", bot);
                    continue;
                }
            };
            let ip = match addr.ip() {
                std::net::IpAddr::V4(ip) => ip.to_ipv6_mapped().octets(),
                std::net::IpAddr::V6(ip) => ip.octets(),
            };

            self.register_addr(Peer {
                ip,
                port: addr.port(),
            })
        }
    }

    pub fn new(data_dir: &std::path::Path) -> Result<Self, AddressManagerError> {
        let mut db_dir = std::path::PathBuf::new();
        db_dir.push(data_dir);
        db_dir.push("adress_manager");
        let db = sled::Db::start_default(db_dir)?;

        Ok(AddressManager { db })
    }

    fn get_peer(&self, peer_address: Peer) -> Result<Option<PeerData>, AddressManagerError> {
        let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
            match self.db.get(peer_address.serialize().to_vec())? {
                Some(b) => (*b).to_owned(),
                None => return Ok(None),
            },
        ));
        PeerData::deserialize(&mut de)
            .map(|a| Some(a))
            .map_err(AddressManagerError::ParseError)
    }

    fn set_peer(&self, peer: Peer, data: PeerData) -> Result<(), AddressManagerError> {
        self.db
            .set(peer.serialize().to_vec(), data.serialize().to_vec())
            .map(|_| ())
            .map_err(AddressManagerError::DbError)
    }

    pub fn get_addr(&self) -> Addr {
        let mut addresses = Vec::new();
        let now = SystemTime::now();
        let since_epoch = now
            .duration_since(UNIX_EPOCH)
            .expect("Back in time are you ?");
        for e in self.db.iter() {
            match e {
                Ok((k, v)) => {
                    let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
                        (*k).to_owned(),
                    ));
                    let peer = Peer::deserialize(&mut de);
                    let mut de = ensicoin_serializer::Deserializer::new(bytes::BytesMut::from(
                        (*v).to_owned(),
                    ));
                    let data = PeerData::deserialize(&mut de);
                    match (peer, data) {
                        (Ok(peer), Ok(data)) => {
                            if data.timestamp + crate::constants::FORGET_TIME
                                > since_epoch.as_secs()
                            {
                                addresses.push(Address {
                                    timestamp: data.timestamp,
                                    ip: peer.ip,
                                    port: peer.port,
                                })
                            } else {
                                if let Err(e) = self.db.del(peer.serialize().to_vec()) {
                                    warn!("Could not delete value in addr db: {}", e)
                                }
                            }
                        }
                        _ => warn!("Error deserializing value in addr db"),
                    }
                }
                Err(e) => warn!("Error reading addr from db: {}", e),
            }
        }
        Addr { addresses }
    }

    fn retime_addr(&self, peer: Peer) {
        let now = SystemTime::now();
        let since_epoch = now
            .duration_since(UNIX_EPOCH)
            .expect("Back in time are you ?");
        let data = match self.get_peer(peer) {
            Ok(data) => data,
            Err(e) => {
                warn!("Error reading from db: {}", e);
                return;
            }
        };
        if let Some(mut data) = data {
            data.timestamp = since_epoch.as_secs();
            if let Err(e) = self.set_peer(peer, data) {
                warn!("Error setting in addr db: {}", e)
            }
        }
    }

    pub fn register_addr(&mut self, peer: Peer) {
        let now = SystemTime::now();
        let since_epoch = now
            .duration_since(UNIX_EPOCH)
            .expect("Back in time are you ?");
        if let Err(e) = self.set_peer(
            peer,
            PeerData {
                timestamp: since_epoch.as_secs(),
            },
        ) {
            warn!("Error registering peer: {}", e)
        }
    }

    pub fn new_message(&mut self, source: &Source) {
        if let Source::Connection(conn) = source {
            self.retime_addr(conn.peer)
        }
    }

    pub fn len(&self) -> usize {
        self.db.len()
    }
}

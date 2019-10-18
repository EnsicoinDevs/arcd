use ensicoin_serializer::types::Sha256Result;
use ensicoin_serializer::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GetBlocks {
    pub block_locator: Vec<Sha256Result>,
    pub stop_hash: Sha256Result,
}

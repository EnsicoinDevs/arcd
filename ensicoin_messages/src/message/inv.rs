use ensicoin_serializer::types::Sha256Result;
use ensicoin_serializer::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct InvVect {
    pub data_type: crate::message::ResourceType,
    pub hash: Sha256Result,
}

impl std::fmt::Debug for InvVect {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ensicoin_serializer::hash_to_string;
        f.debug_struct("InvVect")
            .field("data_type", &self.data_type)
            .field("hash", &hash_to_string(&self.hash))
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Inv {
    pub inventory: Vec<InvVect>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetData {
    pub inventory: Vec<InvVect>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NotFound {
    pub inventory: Vec<InvVect>,
}

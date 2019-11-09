extern crate bytes;

#[cfg(feature = "log")]
#[macro_use]
extern crate log;

pub mod deserializer;
pub mod serializer;
pub mod types;

pub use deserializer::Deserialize;
pub use deserializer::Deserializer;
pub use deserializer::Error;
pub use deserializer::Result;
pub use types::hash_to_string;
pub use types::Sha256Result;
pub use types::VarUint;

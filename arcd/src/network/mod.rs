mod connection;
#[cfg(feature = "grpc")]
mod rpc_server;
mod server;

pub use connection::TerminationReason;
pub use connection::{Connection, State as ConnectionState};
#[cfg(feature = "grpc")]
pub use rpc_server::{node, RPCNode};
pub use server::Server;

use std::time::{SystemTime, UNIX_EPOCH};

pub fn create_self_address(self_port: u16) -> ensicoin_messages::message::Address {
    let now = SystemTime::now();
    let since_epoch = now
        .duration_since(UNIX_EPOCH)
        .expect("Why are we in the past ?");

    ensicoin_messages::message::Address {
        timestamp: since_epoch.as_secs(),
        ip: crate::constants::IP_BYTES,
        port: self_port,
    }
}

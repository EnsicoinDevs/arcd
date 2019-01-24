mod data;
mod network;
use network::Server;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use std::str::FromStr;

fn main() {
    //
    pretty_env_logger::init();
    let mut server = Server::new();
    server.initiate(std::net::IpAddr::from_str("78.248.188.120").unwrap(), 4224);
    server.listen();
}

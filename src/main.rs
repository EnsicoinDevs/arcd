mod data;
mod network;
use network::Server;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use std::str::FromStr;

fn main() {
    pretty_env_logger::init();
    let mut server = Server::new();
    server.initiate(std::net::IpAddr::from_str("127.0.0.1").unwrap(), 7531);
    server.listen();
}

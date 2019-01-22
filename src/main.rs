extern crate ensicoin_serializer;
use ensicoin_serializer::deserializer::Deserialize;
use ensicoin_serializer::deserializer::Deserializer;
use ensicoin_serializer::serializer::Serialize;

mod server;
use server::server::Server;

fn main() {
    let mut server = Server::new();
    server.listen();
}

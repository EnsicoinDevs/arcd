mod data;
mod network;
use network::Server;

fn main() {
    let mut server = Server::new();
    let mut my_stream = std::net::TcpStream::connect("78.248.188.120:4224").unwrap();
    server.initiate(&mut my_stream);
    server.listen();
}

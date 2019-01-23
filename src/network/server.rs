use crate::data::{Message, Whoami};
use std::net;

pub struct Server {
    pub listener: net::TcpListener,
    connections: Vec<net::TcpStream>,
}

impl Server {
    pub fn new() -> Server {
        let server = Server {
            listener: net::TcpListener::bind("127.0.0.1:4224").unwrap(),
            connections: Vec::new(),
        };
        info!("Node started");
        server
    }

    pub fn listen(&mut self) {
        for stream in self.listener.incoming() {
            println!(
                "Connection from : {:?}",
                stream.unwrap().peer_addr().unwrap()
            )
        }
    }

    pub fn initiate(&self, stream: &mut std::net::TcpStream) {
        Whoami::new().send(stream).unwrap();
    }
}

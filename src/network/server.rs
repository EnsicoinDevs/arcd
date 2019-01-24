use std::net;

use crate::network::Connection;

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
            let stream = stream.unwrap().try_clone().unwrap();
            std::thread::spawn(move || {
                let mut conn = Connection::new(stream);
                conn.read_message();
            });
        }
    }

    pub fn initiate(&self, addr: std::net::IpAddr, port: u16) {
        Connection::initiate(addr, port).unwrap();
    }
}

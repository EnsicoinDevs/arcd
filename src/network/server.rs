use std::net;
use std::sync::mpsc;

use crate::network::{Connection, ConnectionMessage};

pub enum ServerMessage {
    Terminate,
}

pub struct Server {
    pub listener: net::TcpListener,
    connection_receiver: mpsc::Receiver<ConnectionMessage>,
    connection_sender: mpsc::Sender<ConnectionMessage>,
    connections: Vec<net::TcpStream>,
}

impl Server {
    pub fn new(port: u16) -> Server {
        let (sender, reciever) = mpsc::channel();
        let server = Server {
            listener: net::TcpListener::bind(("127.0.0.1", port)).unwrap(),
            connections: Vec::new(),
            connection_sender: sender,
            connection_receiver: reciever,
        };
        info!("Node started");
        server
    }

    pub fn listen(&mut self) {
        for stream in self.listener.incoming() {
            let stream = stream.unwrap().try_clone().unwrap();
            let sender = self.connection_sender.clone();
            std::thread::spawn(move || {
                let conn = Connection::new(stream, sender);
                trace!("new connection");
                conn.idle();
            });
        }
    }

    pub fn initiate(&self, addr: std::net::IpAddr, port: u16) {
        if let Err(e) = Connection::initiate(addr, port, self.connection_sender.clone()) {
            error!("Error on connection initiation: {}", e)
        };
    }
}

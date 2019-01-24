use std::net;
use std::sync::mpsc;

use crate::network::{Connection, ConnectionMessage};

pub struct Server {
    pub listener: net::TcpListener,
    connection_reciever: mpsc::Receiver<ConnectionMessage>,
    connection_sender: mpsc::Sender<ConnectionMessage>,
    connections: Vec<net::TcpStream>,
}

impl Server {
    pub fn new() -> Server {
        let (sender, reciever) = mpsc::channel();
        let server = Server {
            listener: net::TcpListener::bind("127.0.0.1:4224").unwrap(),
            connections: Vec::new(),
            connection_sender: sender,
            connection_reciever: reciever,
        };
        info!("Node started");
        server
    }

    pub fn listen(&mut self) {
        for stream in self.listener.incoming() {
            let stream = stream.unwrap().try_clone().unwrap();
            let sender = self.connection_sender.clone();
            std::thread::spawn(move || {
                let mut conn = Connection::new(stream, sender);
                trace!("new connection");
                loop {
                    match conn.read_message() {
                        Ok((message_type, v)) => match conn.handle_message(message_type, v) {
                            Ok(()) => (),
                            _ => {
                                conn.terminate();
                                break;
                            }
                        },
                        _ => {
                            conn.terminate();
                            break;
                        }
                    }
                }
            });
        }
    }

    pub fn initiate(&self, addr: std::net::IpAddr, port: u16) {
        Connection::initiate(addr, port, self.connection_sender.clone()).unwrap();
    }
}

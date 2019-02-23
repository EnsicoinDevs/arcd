use std::net;
use std::sync::mpsc;

use crate::data::MessageType;
use crate::network::{Connection, ConnectionMessage, Error};

#[derive(Debug)]
pub enum ServerMessage {
    Terminate(Error),
    SendMessage(MessageType, Vec<u8>),
    HandleMessage(MessageType, Vec<u8>),
}

pub struct Server {
    connection_receiver: mpsc::Receiver<ConnectionMessage>,
    connections: std::collections::HashMap<String, mpsc::Sender<ServerMessage>>,
    collection_count: u64,
    max_connections_count: u64,
}

impl Server {
    pub fn new(max_conn: u64) -> (Server, mpsc::Sender<ConnectionMessage>) {
        let (sender, reciever) = mpsc::channel();
        let server = Server {
            connections: std::collections::HashMap::new(),
            connection_receiver: reciever,
            collection_count: 0,
            max_connections_count: max_conn,
        };
        info!("Node started");
        (server, sender)
    }

    fn idle(mut self) {
        loop {
            match self.connection_receiver.recv().unwrap() {
                ConnectionMessage::Register(sender, host) => {
                    if self.collection_count < self.max_connections_count {
                        info!("Registered [{}]", &host);
                        self.connections.insert(host, sender);
                        self.collection_count += 1;
                    } else {
                        warn!("Too many connections to accept [{}]", &host);
                        if let Err(_) =
                            sender.send(ServerMessage::Terminate(Error::ServerTermination))
                        {
                            error!("Server can't send messages to [{}]", &host);
                        }
                    }
                }
                ConnectionMessage::Disconnect(e, host) => {
                    if let Some(_) = self.connections.remove(&host) {
                        self.collection_count -= 1;
                    };
                    warn!("Deleted Connection [{}] because of: ({})", host, e);
                }
            }
        }
    }

    pub fn listen(self, port: u16, sender: mpsc::Sender<ConnectionMessage>) {
        let listener = net::TcpListener::bind(("127.0.0.1", port)).unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = stream.unwrap().try_clone().unwrap();
                let sender = sender.clone();
                std::thread::spawn(move || {
                    let conn = Connection::new(stream, sender.clone());
                    trace!("new connection");
                    conn.idle();
                });
            }
        });
        self.idle();
    }

    pub fn initiate(
        &self,
        addr: std::net::IpAddr,
        port: u16,
        sender: mpsc::Sender<ConnectionMessage>,
    ) {
        if let Err(e) = Connection::initiate(addr, port, sender) {
            error!("Error on connection initiation: {}", e)
        };
    }
}

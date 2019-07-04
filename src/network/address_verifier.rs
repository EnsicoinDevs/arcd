use crate::data::intern_messages::{ConnectionMessage, ConnectionMessageContent, Source};
use ensicoin_messages::message::{Addr, Address};
use futures::prelude::*;
use futures::sync::mpsc;
use std::net::SocketAddr;
use tokio::net::TcpStream;

fn verify_connection(address: Address, sender: mpsc::Sender<ConnectionMessage>) {
    tokio::spawn(
        TcpStream::connect(&SocketAddr::from((address.ip, address.port)))
            .then(move |r| match r {
                Err(_) => Err(()),
                Ok(_) => Ok(sender.send(ConnectionMessage {
                    content: ConnectionMessageContent::VerifiedAddr(address),
                    source: Source::Server,
                })),
            })
            .map_err(|e| debug!("Could not process: {:?}", e))
            .map(|_| ()),
    );
}

pub fn verify_addr(addr: Addr, sender: mpsc::Sender<ConnectionMessage>) {
    for addr in addr.addresses {
        verify_connection(addr, sender.clone())
    }
}

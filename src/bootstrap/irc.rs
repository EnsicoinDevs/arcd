use crate::constants;
use crate::data::intern_messages::ConnectionMessage;
use std::net::ToSocketAddrs;
use tokio::codec::LinesCodec;
use tokio::net::TcpStream;
use tokio::prelude::*;

#[derive(Debug)]
pub enum IrcError {
    IoError(std::io::Error),
    UnresolvedHostname,
}

impl From<std::io::Error> for IrcError {
    fn from(err: std::io::Error) -> IrcError {
        IrcError::IoError(err)
    }
}

pub fn irc_listener(
    _server_sender: futures::sync::mpsc::Sender<ConnectionMessage>,
) -> Result<impl Future<Item = (), Error = ()>, IrcError> {
    let freenode_addr = match "irc.freenode.net:6667".to_socket_addrs()?.next() {
        Some(a) => a,
        None => return Err(IrcError::UnresolvedHostname),
    };
    let nick = format!(
        "{}_{}:{}",
        constants::MAGIC,
        constants::IP,
        constants::DEFAULT_PORT
    );
    Ok(TcpStream::connect(&freenode_addr)
        .map(|stream| tokio::codec::Framed::new(stream, LinesCodec::new()))
        .map(move |frame| {
            frame
                .send_all(stream::iter_ok::<_, std::io::Error>(vec![
                    format!("NICK {}", nick),
                    format!("USER {} 0 x : {}", nick, nick),
                    format!("JOIN #ensicoin"),
                ]))
                .and_then(|(_, stream)| {
                    info!("Connected to the IRC channel");
                    stream.for_each(|message| {
                        println!("Message: {}", message);
                        Ok(())
                    })
                })
        })
        .map_err(|err| println!("Err: {}", err))
        .and_then(|_| Ok(())))
}

use crate::constants;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::ToSocketAddrs;
use std::net::{SocketAddr, TcpStream};

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

pub fn irc_listener() -> Result<(TcpStream, Vec<SocketAddr>), IrcError> {
    let freenode_addr = match "irc.freenode.net:6667".to_socket_addrs()?.next() {
        Some(a) => a,
        None => return Err(IrcError::UnresolvedHostname),
    };
    let nick = format!(
        "XEC{}_{:x}",
        constants::HEX_IP,
        constants::DEFAULT_PORT.parse::<u16>().unwrap(),
    );
    let mut irc_stream = TcpStream::connect(&freenode_addr)?;
    irc_stream.write(format!("NICK {}\r\n", nick).as_bytes())?;
    irc_stream.write(format!("USER {} 0 x : {}\r\n", nick, nick).as_bytes())?;
    irc_stream.write(format!("JOIN #ensicoin\r\n").as_bytes())?;
    let reader = BufReader::new(irc_stream.try_clone()?);

    let mut irc_peers = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let mut split_line = line.split(" ");
        let reply_slot = split_line.nth(1);
        if let Some("353") = reply_slot {
            for user in split_line.skip(4) {
                if user.starts_with("XEC") {
                    println!("Remote: {}", user);
                    irc_peers.push(user.clone().to_owned());
                }
            }
        } else if let Some("366") = reply_slot {
            debug!("[IRC] finished reading names");
            break;
        } else {
            trace!("[IRC] {}", line);
        }
    }
    info!("[IRC] Found {} peers", irc_peers.len());
    Ok((irc_stream, Vec::new()))
}

use futures::sync::mpsc;
use tokio::net::TcpListener;
use tokio::prelude::*;

use crate::data::ressources::LinkedTransaction;
use crate::manager::{Mempool, UtxoManager};
use crate::network::{Connection, ConnectionMessage, ServerMessage};
use crate::Error;

const CHANNEL_CAPACITY: usize = 1_024;

pub struct Server {
    connection_receiver: FullMessageStream,
    connection_sender: mpsc::Sender<ConnectionMessage>,
    connections: std::collections::HashMap<String, mpsc::Sender<ServerMessage>>,
    connection_buffer: std::collections::VecDeque<(String, ServerMessage)>,
    utxo_manager: UtxoManager,
    mempool: Mempool,
    collection_count: u64,
    max_connections_count: u64,
}

impl futures::Future for Server {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.connection_receiver.poll() {
                Ok(Async::Ready(None)) => (),
                Ok(Async::Ready(Some(msg))) => self.handle_message(msg),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => panic!("Server encountered an error: {}", e),
            }
            while !self.connection_buffer.is_empty() {
                let (dest, msg) = self.connection_buffer.pop_front().unwrap();
                match self.connections.get_mut(&dest).unwrap().start_send(msg) {
                    Ok(AsyncSink::NotReady(msg)) => self.connection_buffer.push_front((dest, msg)),
                    Err(e) => warn!("Can't concat [{}] connection: {}", dest, e),
                    Ok(AsyncSink::Ready) => {
                        match self.connections.get_mut(&dest).unwrap().poll_complete() {
                            Ok(Async::Ready(_)) => (),
                            Ok(Async::NotReady) => return Ok(Async::NotReady),
                            Err(e) => warn!("Can't contact [{}] connection: {}", dest, e),
                        }
                    }
                }
            }
        }
    }
}

fn new_socket_converter(socket: tokio::net::TcpStream) -> ConnectionMessage {
    trace!(
        "Connection picked up: {}",
        socket.peer_addr().unwrap().to_string()
    );
    ConnectionMessage::NewConnection(socket)
}
fn io_error_converter(e: std::io::Error) -> Error {
    Error::from(e)
}
fn channel_errore_converter(_: ()) -> Error {
    Error::ChannelError
}

type NewSocketConverter = fn(tokio::net::TcpStream) -> ConnectionMessage;
type NewConnectionStream = futures::stream::Map<tokio::net::tcp::Incoming, NewSocketConverter>;
type IoErrorConverter = fn(std::io::Error) -> Error;
type NewConnectionStreamErrored = futures::stream::MapErr<NewConnectionStream, IoErrorConverter>;
type ChannelErrorConverter = fn(()) -> Error;
type ServerMessageErrored = futures::stream::MapErr<
    futures::sync::mpsc::Receiver<ConnectionMessage>,
    ChannelErrorConverter,
>;
type FullMessageStream = futures::stream::Select<ServerMessageErrored, NewConnectionStreamErrored>;

impl Server {
    pub fn new(max_conn: u64, data_dir: &std::path::Path, port: u16) -> Server {
        let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);

        let listener =
            TcpListener::bind(&std::net::SocketAddr::new("0.0.0.0".parse().unwrap(), port))
                .unwrap();
        let inbound_connection_stream = listener
            .incoming()
            .map(new_socket_converter as NewSocketConverter)
            .map_err(io_error_converter as IoErrorConverter);
        let message_stream = receiver
            .map_err(channel_errore_converter as ChannelErrorConverter)
            .select(inbound_connection_stream);

        let server = Server {
            connections: std::collections::HashMap::new(),
            connection_receiver: message_stream,
            connection_sender: sender,
            collection_count: 0,
            max_connections_count: max_conn,
            utxo_manager: UtxoManager::new(data_dir),
            mempool: Mempool::new(),
            connection_buffer: std::collections::VecDeque::new(),
        };
        info!("Node started");
        server
    }

    fn handle_message(&mut self, message: ConnectionMessage) {
        trace!("Server handling: {}", message);
        match message {
            ConnectionMessage::Register(sender, host) => {
                if self.collection_count < self.max_connections_count {
                    info!("Registered [{}]", &host);
                    self.connections.insert(host, sender);
                    self.collection_count += 1;
                } else {
                    warn!("Too many connections to accept [{}]", &host);
                    tokio::spawn(
                        sender
                            .send(ServerMessage::Terminate(Error::ServerTermination))
                            .map(|_| ())
                            .map_err(|_| ()),
                    );
                }
            }
            ConnectionMessage::Disconnect(e, host) => {
                if let Some(_) = self.connections.remove(&host) {
                    self.collection_count -= 1;
                };
                warn!("Deleted Connection [{}] because of: ({})", host, e);
            }
            ConnectionMessage::CheckInv(_, _) => (),
            ConnectionMessage::Retrieve(_, _) => (),
            ConnectionMessage::SyncBlocks(_, _) => (),
            ConnectionMessage::NewTransaction(tx) => {
                let mut ltx = LinkedTransaction::new(tx);
                self.utxo_manager.link(&mut ltx);
                self.mempool.insert(ltx);
            }
            ConnectionMessage::NewConnection(socket) => {
                // TODO: add connection limit
                let new_conn = Connection::new(socket, self.connection_sender.clone());
                trace!("new connection");
                tokio::spawn(new_conn);
            }
        }
    }
    pub fn get_sender(&self) -> mpsc::Sender<ConnectionMessage> {
        self.connection_sender.clone()
    }
}

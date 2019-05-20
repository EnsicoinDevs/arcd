use ensicoin_messages::message::Message;
use futures::sync::mpsc;
use tokio::{net::TcpListener, prelude::*};
use tokio_bus::Bus;

use crate::{
    data::{
        intern_messages::{BroadcastMessage, ConnectionMessage, ServerMessage},
        linkedblock::LinkedBlock,
        linkedtx::LinkedTransaction,
    },
    manager::{Blockchain, Mempool, NewAddition, UtxoManager},
    network::{Connection, RPCNode},
    Error,
};
use std::sync::{Arc, RwLock};

const CHANNEL_CAPACITY: usize = 1_024;

pub struct Server {
    broadcast_channel: Arc<RwLock<Bus<BroadcastMessage>>>,
    connection_receiver: FullMessageStream,
    connection_sender: mpsc::Sender<ConnectionMessage>,
    connections: std::collections::HashMap<String, mpsc::Sender<ServerMessage>>,
    connection_buffer: std::collections::VecDeque<(String, ServerMessage)>,
    utxo_manager: UtxoManager,
    blockchain: Arc<RwLock<Blockchain>>,
    mempool: Arc<RwLock<Mempool>>,
    collection_count: u64,
    max_connections_count: u64,
    sync_counter: u64,
}

impl futures::Future for Server {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.connection_receiver.poll() {
                Ok(Async::Ready(None)) => (),
                Ok(Async::Ready(Some(msg))) => self.handle_message(msg).unwrap(),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => panic!("Server encountered an error: {}", e),
            }
            while !self.connection_buffer.is_empty() {
                let (dest, msg) = self.connection_buffer.pop_front().unwrap();
                match self.connections.get_mut(&dest).unwrap().start_send(msg) {
                    Ok(AsyncSink::NotReady(msg)) => self.connection_buffer.push_front((dest, msg)),
                    Err(e) => warn!("Can't conctact [{}] connection: {}", dest, e),
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
    fn send(&mut self, dest: String, msg: crate::data::intern_messages::ServerMessage) {
        self.connection_buffer.push_back((dest, msg));
    }

    pub fn run(
        max_conn: u64,
        data_dir: &std::path::Path,
        port: u16,
        grpc_port: u16,
        grpc_localhost: bool,
    ) {
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
            broadcast_channel: Arc::new(RwLock::from(Bus::new(30))),
            connections: std::collections::HashMap::new(),
            connection_receiver: message_stream,
            connection_sender: sender,
            collection_count: 0,
            max_connections_count: max_conn,
            utxo_manager: UtxoManager::new(data_dir),
            blockchain: Arc::new(RwLock::new(Blockchain::new(data_dir))),
            mempool: Arc::new(RwLock::new(Mempool::new())),
            connection_buffer: std::collections::VecDeque::new(),
            sync_counter: 3,
        };
        info!("Node created, listening on port {}", port);
        let rpc = RPCNode::new(
            server.broadcast_channel.clone(),
            server.mempool.clone(),
            server.blockchain.clone(),
            server.connection_sender.clone(),
            if grpc_localhost {
                "127.0.0.1"
            } else {
                "0.0.0.0"
            },
            grpc_port,
        );
        tokio::run(rpc.join(server).map(|_| ()));
    }

    fn handle_message(&mut self, message: ConnectionMessage) -> Result<(), Error> {
        debug!("Server handling: {}", message);
        match message {
            ConnectionMessage::Register(sender, host) => {
                if self.collection_count < self.max_connections_count {
                    info!("Registered [{}]", &host);
                    if self.sync_counter > 0 {
                        self.sync_counter -= 1;
                        let getblocks = self.blockchain.read()?.generate_get_blocks()?;
                        let (t, v) = getblocks.raw_bytes();
                        let msg = ServerMessage::SendMessage(t, v);
                        let getmempool = ensicoin_messages::message::GetMempool {};
                        let (t, v) = getmempool.raw_bytes();
                        let msg_get_mempool = ServerMessage::SendMessage(t, v);
                        self.send(host.clone(), msg);
                        self.send(host.clone(), msg_get_mempool);
                    };
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
            ConnectionMessage::Clean(host) => {
                if let Some(_) = self.connections.remove(&host) {
                    self.collection_count -= 1;
                };
                trace!("Cleaned connection [{}]", host);
            }
            ConnectionMessage::Disconnect(e, host) => {
                if self.connections.contains_key(&host) {
                    self.send(host, ServerMessage::Terminate(e));
                }
            }
            ConnectionMessage::CheckInv(inv, source) => {
                let (mut unknown, txs) =
                    self.blockchain.read()?.get_unknown_blocks(inv.inventory)?;
                let (mut unknown_tx, _) = self.mempool.read()?.get_unknown_tx(txs);
                unknown.append(&mut unknown_tx);
                let get_data = ensicoin_messages::message::GetData { inventory: unknown };
                if get_data.inventory.len() > 0 {
                    match source {
                        crate::data::intern_messages::Source::Connection(remote) => {
                            let (t, v) = get_data.raw_bytes();
                            self.send(remote, ServerMessage::SendMessage(t, v));
                        }
                        _ => (),
                    }
                };
            }
            ConnectionMessage::Retrieve(_, _) => (), //TODO: handle getdata
            ConnectionMessage::SyncBlocks(get_blocks, remote) => {
                let inv = self.blockchain.read()?.generate_inv(&get_blocks)?;
                if inv.inventory.len() > 0 {
                    match remote {
                        crate::data::intern_messages::Source::Connection(remote) => {
                            let (t, v) = inv.raw_bytes();
                            self.send(remote, ServerMessage::SendMessage(t, v));
                        }
                        _ => (),
                    }
                }
            }
            ConnectionMessage::Connect(address) => {
                Connection::initiate(&address, self.connection_sender.clone());
            }
            ConnectionMessage::NewTransaction(tx, _) => {
                // Verify tx in mempool insert
                let mut ltx = LinkedTransaction::new(tx);
                self.utxo_manager.link(&mut ltx);
                self.mempool.write().unwrap().insert(ltx);
            }
            ConnectionMessage::NewBlock(block, source) => {
                let mut lblock = LinkedBlock::new(block);
                let hash = lblock.header.double_hash();
                self.utxo_manager.link_block(&mut lblock);
                let new_target = self
                    .blockchain
                    .read()?
                    .get_target_next_block(lblock.header.timestamp)?;
                debug!(
                    "Validating block {}",
                    ensicoin_serializer::hash_to_string(&lblock.header.double_hash())
                );
                if lblock.is_valid(new_target) {
                    let addition = self.blockchain.write()?.new_block(lblock.clone())?;
                    match addition {
                        NewAddition::Fork => {
                            debug!("Handling fork");
                            self.utxo_manager.register_block(&lblock)?;
                            self.mempool.write()?.remove_tx(&lblock);
                            let best_block = self.blockchain.read()?.best_block_hash()?;
                            let common_hash = match self
                                .blockchain
                                .read()?
                                .find_common_hash(best_block, hash.clone())?
                            {
                                Some(h) => h,
                                None => return Err(Error::NotFound("merge point".to_string())),
                            };
                            let new_branch =
                                self.blockchain.read()?.chain_until(&hash, &common_hash)?;
                            let pop_contex = self.blockchain.write()?.pop_until(&common_hash)?;
                            for utxo in pop_contex.utxo_to_remove {
                                self.utxo_manager.delete(&utxo)?;
                            }
                            self.utxo_manager.restore(pop_contex.utxo_to_restore)?;
                            for tx in pop_contex.txs_to_restore {
                                let mut ltx = LinkedTransaction::new(tx);
                                self.utxo_manager.link(&mut ltx);
                                self.mempool.write()?.insert(ltx);
                            }
                            let block_chain =
                                self.blockchain.read()?.chain_to_blocks(new_branch)?;
                            let mut linked_chain: Vec<_> = block_chain
                                .into_iter()
                                .map(|b| LinkedBlock::new(b))
                                .collect();
                            linked_chain
                                .iter_mut()
                                .for_each(|mut lb| self.utxo_manager.link_block(&mut lb));
                            for lb in &linked_chain {
                                self.utxo_manager.register_block(lb)?;
                            }
                            self.blockchain.write()?.add_chain(linked_chain)?;
                            if let Err(_) = self.broadcast_channel.write()?.try_broadcast(
                                BroadcastMessage::BestBlock(
                                    self.blockchain
                                        .read()?
                                        .get_block(&self.blockchain.read()?.best_block_hash()?)?
                                        .unwrap(),
                                ),
                            ) {
                                error!("Could not broadcast");
                            }
                        }
                        NewAddition::BestBlock => {
                            debug!("Adding as best block");
                            self.utxo_manager.register_block(&lblock)?;
                            if let Err(_) = self.broadcast_channel.write()?.try_broadcast(
                                BroadcastMessage::BestBlock(
                                    self.blockchain
                                        .read()?
                                        .get_block(&self.blockchain.read()?.best_block_hash()?)?
                                        .unwrap(),
                                ),
                            ) {
                                error!("Could not broadcast");
                            }
                        }
                        _ => (),
                    }
                } else {
                    warn!("Recieved invalid Block from {}", source);
                }
            }
            ConnectionMessage::NewConnection(socket) => {
                if self.collection_count < self.max_connections_count {
                    let new_conn = Connection::new(socket, self.connection_sender.clone());
                    trace!("new connection");
                    tokio::spawn(new_conn);
                }
            }
        }
        Ok(())
    }
}

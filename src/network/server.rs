use crate::bootstrap::matrix;
use ensicoin_messages::message::Message;
use futures::sync::mpsc;
use tokio::{net::TcpListener, prelude::*};
use tokio_bus::Bus;

use crate::{
    data::{
        intern_messages::{
            BroadcastMessage, ConnectionMessage, ConnectionMessageContent, ServerMessage, Source,
        },
        linkedblock::LinkedBlock,
        linkedtx::LinkedTransaction,
    },
    manager::{AddressManager, Blockchain, Mempool, NewAddition, OrphanBlockManager, UtxoManager},
    network::{Connection, RPCNode},
    Error,
};
use std::sync::{Arc, RwLock};

const CHANNEL_CAPACITY: usize = 2_048;

pub struct Server {
    broadcast_channel: Arc<RwLock<Bus<BroadcastMessage>>>,

    connection_receiver: Box<dyn futures::Stream<Item = ConnectionMessage, Error = Error> + Send>,
    connection_sender: mpsc::Sender<ConnectionMessage>,

    connections: std::collections::HashMap<String, mpsc::Sender<ServerMessage>>,
    connection_buffer: std::collections::VecDeque<(String, ServerMessage)>,

    utxo_manager: UtxoManager,
    blockchain: Arc<RwLock<Blockchain>>,
    mempool: Arc<RwLock<Mempool>>,

    address_manager: AddressManager,
    collection_count: u64,
    max_connections_count: u64,

    sync_counter: u64,

    orphan_manager: OrphanBlockManager,

    matrix_client: Option<matrix::MatrixClient>,

    origin_port: u16,
}

impl futures::Future for Server {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.connection_receiver.poll() {
                Ok(Async::Ready(None)) => (),
                Ok(Async::Ready(Some(msg))) => match self.handle_message(msg) {
                    Ok(false) => return Ok(Async::Ready(())),
                    Err(e) => {
                        error!("The server shutdown due to an error: {}", e);
                        return Err(());
                    }
                    _ => (),
                },
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => {
                    error!("Server encountered an error: {}", e);
                    return Err(());
                }
            }
            if self.collection_count == 0 {
                self.find_new_peer();
            };
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
        matrix_config: Option<matrix::Config>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);

        let listener =
            TcpListener::bind(&std::net::SocketAddr::new("0.0.0.0".parse().unwrap(), port))?;
        let inbound_connection_stream = listener
            .incoming()
            .map(|socket| {
                trace!(
                    "Picked up connection: {}",
                    socket.peer_addr().unwrap().to_string()
                );
                ConnectionMessage {
                    content: ConnectionMessageContent::NewConnection(socket),
                    source: Source::Server,
                }
            })
            .map_err(Error::from);
        let message_stream = Box::new(
            receiver
                .map_err(|_| Error::ChannelError)
                .select(inbound_connection_stream)
                .select(
                    tokio_signal::ctrl_c()
                        .flatten_stream()
                        .map(|_| ConnectionMessage {
                            content: ConnectionMessageContent::Quit,
                            source: Source::Server,
                        })
                        .map_err(|_| Error::SignalError),
                ),
        );

        let broadcast_channel = Arc::new(RwLock::from(Bus::new(30)));

        let address_manager = AddressManager::new(data_dir)?;

        let mut server = Server {
            broadcast_channel,
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
            orphan_manager: OrphanBlockManager::new(),
            matrix_client: None,
            address_manager,
            origin_port: port,
        };
        info!("Node created, listening on port {}", port);
        let rpc = RPCNode::server(
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
        let mut initial_peers = Vec::new();
        if let Some(config) = matrix_config {
            let matrix_client = matrix::MatrixClient::new(config);
            match matrix_client.get_room_id() {
                Ok(Some(room_id)) => {
                    if let Err(e) = matrix_client.set_status(&matrix::Status::Online) {
                        warn!("Could not pass online on matrix: {}", e)
                    } else {
                        if let Err(e) = matrix_client.set_name(
                            &format!("{}", crate::constants::MAGIC),
                            crate::constants::IP,
                            &format!("{}", port),
                        ) {
                            warn!("Could not set matrix displayname: {}", e)
                        } else {
                            match matrix_client
                                .get_bots(&room_id, &format!("{}", crate::constants::MAGIC))
                            {
                                Ok(b) => initial_peers = b,
                                Err(e) => warn!("Could not retrieve initial_peers: {}", e),
                            }
                        }
                    }
                }
                Ok(None) => warn!("No room id in matrix response"),
                Err(e) => warn!("Could not get matrix room id: {}", e),
            }
            server.matrix_client = Some(matrix_client);
        }
        info!(
            "Starting server with {} peers from matrix and {} from known peers",
            initial_peers.len(),
            server.address_manager.len()
        );
        server.address_manager.set_bots(initial_peers);
        tokio::run(rpc.select(server).map_err(|_| ()).map(|_| ()));
        Ok(())
    }

    fn broadcast_to_connections(&mut self, message: ServerMessage) {
        let remotes: Vec<_> = self.connections.keys().cloned().collect();
        for remote in remotes {
            self.send(remote.clone(), message.clone());
        }
    }

    // The boolean means should execution continue or not
    fn handle_message(&mut self, message: ConnectionMessage) -> Result<bool, Error> {
        debug!("Server handling {}", message);
        self.address_manager.new_message(&message.source);
        match message.content {
            ConnectionMessageContent::ConnectionFailed(address) => {
                info!("Connection {} failed", address);
            }
            ConnectionMessageContent::Quit => {
                if let Some(matrix_client) = self.matrix_client.take() {
                    info!("Offline in matrix");
                    matrix::async_set_status(matrix_client.config(), &matrix::Status::Offline);
                }
                info!("Shuting down RPC server");
                if self
                    .broadcast_channel
                    .write()?
                    .try_broadcast(BroadcastMessage::Quit)
                    .is_err()
                {
                    error!("Cannot stop RPC server")
                }
                info!("Disconnecting Peers");
                for conn_sender in self.connections.values_mut() {
                    tokio::spawn(
                        conn_sender
                            .clone()
                            .send(ServerMessage::Terminate(Error::ServerTermination))
                            .map(|_| ())
                            .map_err(|e| warn!("Failed to bring down connection: {}", e)),
                    );
                }
                info!("Node shutdown !");
                return Ok(false);
            }
            ConnectionMessageContent::RetrieveAddr => {
                if let Source::Connection(conn) = message.source {
                    let (t, v) = self.address_manager.get_addr().raw_bytes();
                    self.send(conn.tcp_address, ServerMessage::SendMessage(t, v));
                }
            }
            ConnectionMessageContent::NewAddr(addr) => {
                crate::network::verify_addr(addr, self.connection_sender.clone());
            }
            ConnectionMessageContent::VerifiedAddr(address) => (), // TODO: register addr
            ConnectionMessageContent::Register(sender, host) => {
                if self.collection_count < self.max_connections_count {
                    info!("Registered [{}]", &host.tcp_address);
                    if self.sync_counter > 0 {
                        self.sync_counter -= 1;
                        let getblocks = self.blockchain.read()?.generate_get_blocks()?;
                        let (t, v) = getblocks.raw_bytes();
                        let msg = ServerMessage::SendMessage(t, v);
                        let getmempool = ensicoin_messages::message::GetMempool {};
                        let (t, v) = getmempool.raw_bytes();
                        let msg_get_mempool = ServerMessage::SendMessage(t, v);
                        self.send(host.tcp_address.clone(), msg);
                        self.send(host.tcp_address.clone(), msg_get_mempool);
                    };
                    self.connections.insert(host.tcp_address, sender);
                    self.address_manager.register_addr(host.peer);
                    self.collection_count += 1;
                } else {
                    warn!("Too many connections to accept [{}]", &host.tcp_address);
                    tokio::spawn(
                        sender
                            .send(ServerMessage::Terminate(Error::ServerTermination))
                            .map(|_| ())
                            .map_err(|_| ()),
                    );
                }
            }
            ConnectionMessageContent::Clean(host) => {
                if self.connections.remove(&host.tcp_address).is_some() {
                    self.collection_count -= 1;
                };
                trace!("Cleaned connection [{}]", host.tcp_address);
            }
            ConnectionMessageContent::Disconnect(e, host) => {
                if self.connections.contains_key(&host) {
                    self.send(host, ServerMessage::Terminate(e));
                }
            }
            ConnectionMessageContent::CheckInv(inv) => {
                let (mut unknown, txs) =
                    self.blockchain.read()?.get_unknown_blocks(inv.inventory)?;
                let (mut unknown_tx, _) = self.mempool.read()?.get_unknown_tx(txs);
                unknown.append(&mut unknown_tx);
                let get_data = ensicoin_messages::message::GetData { inventory: unknown };
                if !get_data.inventory.is_empty() {
                    if let crate::data::intern_messages::Source::Connection(remote) = message.source
                    {
                        let (t, v) = get_data.raw_bytes();
                        self.send(remote.tcp_address, ServerMessage::SendMessage(t, v));
                    }
                };
            }
            ConnectionMessageContent::Retrieve(get_data) => {
                // GetData
                if let crate::data::intern_messages::Source::Connection(remote) = message.source {
                    let (blocks, remaining) =
                        self.blockchain.read()?.get_data(get_data.inventory)?;
                    for block in blocks {
                        let (t, v) = block.raw_bytes();
                        self.send(remote.tcp_address.clone(), ServerMessage::SendMessage(t, v));
                    }
                    let (txs, _) = self.mempool.read()?.get_data(remaining);
                    for tx in txs {
                        let (t, v) = tx.raw_bytes();
                        self.send(remote.tcp_address.clone(), ServerMessage::SendMessage(t, v));
                    }
                }
            }
            ConnectionMessageContent::SyncBlocks(get_blocks) => {
                // Handling: Best Block
                let inv = self.blockchain.read()?.generate_inv(&get_blocks)?;
                if !inv.inventory.is_empty() {
                    if let crate::data::intern_messages::Source::Connection(remote) = message.source
                    {
                        let (t, v) = inv.raw_bytes();
                        self.send(remote.tcp_address, ServerMessage::SendMessage(t, v));
                    }
                }
            }
            ConnectionMessageContent::Connect(address) => {
                Connection::initiate(address, self.connection_sender.clone(), self.origin_port);
            }
            ConnectionMessageContent::NewTransaction(tx) => {
                // TODO: Verify tx in mempool insert
                let mut ltx = LinkedTransaction::new(tx);
                self.utxo_manager.link(&mut ltx);
                self.mempool.write().unwrap().insert(ltx);
            }
            ConnectionMessageContent::NewBlock(block) => {
                self.handle_new_block(block, message.source)?;
            }
            ConnectionMessageContent::NewConnection(socket) => {
                if self.collection_count < self.max_connections_count {
                    let new_conn =
                        Connection::new(socket, self.connection_sender.clone(), self.origin_port);
                    trace!("new connection");
                    tokio::spawn(new_conn);
                }
            }
        }
        Ok(true)
    }

    // TODO: Be a good peer finder
    fn find_new_peer(&mut self) {}

    fn handle_new_block(
        &mut self,
        block: ensicoin_messages::resource::Block,
        source: crate::data::intern_messages::Source,
    ) -> Result<(), Error> {
        info!("Handling block of height: {}", block.header.height);
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
        let prev_block = match self
            .blockchain
            .read()?
            .get_block(&lblock.header.prev_block)?
        {
            Some(b) => b,
            None => {
                warn!(
                    "Orphan block: {}",
                    ensicoin_serializer::hash_to_string(&lblock.header.double_hash())
                );
                self.orphan_manager.add_block((source, lblock.into_block()));
                return Ok(());
            }
        };
        if lblock.is_valid(new_target, prev_block.header.height) {
            let (t, v) = ensicoin_messages::message::Inv {
                inventory: vec![ensicoin_messages::message::InvVect {
                    hash: lblock.header.double_hash(),
                    data_type: ensicoin_messages::message::ResourceType::Block,
                }],
            }
            .raw_bytes();
            self.broadcast_to_connections(ServerMessage::SendMessage(t, v));
            let addition = self.blockchain.write()?.new_block(lblock.clone())?;
            match addition {
                NewAddition::Fork => {
                    info!("Handling fork");
                    self.utxo_manager.register_block(&lblock)?;
                    self.mempool.write()?.remove_tx(&lblock);
                    let best_block = self.blockchain.read()?.best_block_hash()?;
                    let common_hash =
                        match self.blockchain.read()?.find_common_hash(best_block, hash)? {
                            Some(h) => h,
                            None => return Err(Error::NotFound("merge point".to_string())),
                        };
                    let new_branch = self.blockchain.read()?.chain_until(&hash, &common_hash)?;
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
                    let block_chain = self.blockchain.read()?.chain_to_blocks(new_branch)?;
                    let mut linked_chain: Vec<_> =
                        block_chain.into_iter().map(LinkedBlock::new).collect();
                    linked_chain
                        .iter_mut()
                        .for_each(|mut lb| self.utxo_manager.link_block(&mut lb));
                    for lb in &linked_chain {
                        self.utxo_manager.register_block(lb)?;
                    }
                    self.blockchain.write()?.add_chain(linked_chain)?;
                    trace!(
                        "New best block after fork: {}",
                        ensicoin_serializer::hash_to_string(&lblock.header.double_hash())
                    );
                    if self
                        .broadcast_channel
                        .write()?
                        .try_broadcast(BroadcastMessage::BestBlock(
                            self.blockchain
                                .read()?
                                .get_block(&self.blockchain.read()?.best_block_hash()?)?
                                .unwrap(),
                        ))
                        .is_err()
                    {
                        error!("Could not broadcast");
                    }
                }
                NewAddition::BestBlock => {
                    trace!(
                        "New best block: {}",
                        ensicoin_serializer::hash_to_string(&lblock.header.double_hash())
                    );
                    self.utxo_manager.register_block(&lblock)?;
                    if self
                        .broadcast_channel
                        .write()?
                        .try_broadcast(BroadcastMessage::BestBlock(
                            self.blockchain
                                .read()?
                                .get_block(&self.blockchain.read()?.best_block_hash()?)?
                                .unwrap(),
                        ))
                        .is_err()
                    {
                        error!("Could not broadcast");
                    }
                }
                NewAddition::Nothing => {
                    info!("Added block to a sidechain");
                }
            }
        } else {
            warn!("Recieved invalid Block from {}", source);
        }
        let best_block_hash = self.blockchain.read()?.best_block_hash()?;
        let orphan_chain = self.orphan_manager.retrieve_chain(best_block_hash);
        for (s, b) in orphan_chain {
            self.handle_new_block(b, s)?;
        }
        Ok(())
    }
}

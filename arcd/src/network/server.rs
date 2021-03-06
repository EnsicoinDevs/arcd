#[cfg(feature = "matrix_discover")]
use crate::bootstrap::matrix;
use ensicoin_messages::message::Message;
use tokio::{
    net::TcpListener,
    prelude::*,
    sync::{mpsc, watch},
};

#[cfg(feature = "grpc")]
use crate::data::intern_messages::BroadcastMessage;
#[cfg(feature = "grpc")]
use crate::network::RPCNode;
use crate::{
    data::{
        intern_messages::{ConnectionMessage, ConnectionMessageContent, ServerMessage, Source},
        linkedblock::LinkedBlock,
        linkedtx::LinkedTransaction,
    },
    manager::{AddressManager, Blockchain, Mempool, NewAddition, OrphanBlockManager, UtxoManager},
    network::{Connection, TerminationReason},
    Error, ServerConfig,
};
#[cfg(feature = "grpc")]
use futures::future::{AbortHandle, Abortable, Aborted, TryFutureExt};
#[cfg(feature = "grpc")]
use std::sync::Arc;
#[cfg(feature = "grpc")]
use tokio::sync::Mutex;

const CHANNEL_CAPACITY: usize = 2_048;

pub struct Server {
    #[cfg(feature = "grpc")]
    broadcast_channel_tx: watch::Sender<BroadcastMessage>,
    #[cfg(feature = "grpc")]
    rpc_abort: futures::future::AbortHandle,

    connection_receiver: mpsc::Receiver<ConnectionMessage>,
    connection_sender: mpsc::Sender<ConnectionMessage>,

    connections: std::collections::HashMap<u64, mpsc::Sender<ServerMessage>>,

    utxo_manager: UtxoManager,
    #[cfg(feature = "grpc")]
    blockchain: Arc<Mutex<Blockchain>>,
    #[cfg(not(feature = "grpc"))]
    blockchain: Blockchain,
    #[cfg(not(feature = "grpc"))]
    mempool: Mempool,
    #[cfg(feature = "grpc")]
    mempool: Arc<Mutex<Mempool>>,

    address_manager: AddressManager,
    connection_count: u64,
    max_connections_count: u64,

    sync_counter: u64,

    next_id_to_give: u64,

    orphan_manager: OrphanBlockManager,

    #[cfg(feature = "matrix_discover")]
    matrix_client: Option<matrix::MatrixClient>,

    origin_port: u16,
}

impl Server {
    pub async fn run(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
        let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);

        let mut listener = TcpListener::bind(&std::net::SocketAddr::new(
            "0.0.0.0".parse().unwrap(),
            config.port,
        ))
        .await?;
        let mut sender_clone = sender.clone();
        tokio::spawn(async move {
            loop {
                let (socket, addr) = match listener.accept().await {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Error accepting connection: {:?}", e);
                        continue;
                    }
                };
                trace!("Picked up connection: {}", addr.to_string());
                if let Err(e) = sender_clone
                    .send(ConnectionMessage {
                        content: ConnectionMessageContent::NewConnection(socket),
                        source: Source::Server,
                    })
                    .await
                {
                    warn!("Could not notify server of new connection: {:?}", e);
                    break;
                }
            }
        });
        let mut sender_clone = sender.clone();
        tokio::spawn(async move {
            let mut ctrl_c =
                tokio::net::signal::ctrl_c().expect("Could not register signal handler");
            ctrl_c.next().await;
            sender_clone
                .send(ConnectionMessage {
                    content: ConnectionMessageContent::Quit,
                    source: Source::Server,
                })
                .await
                .expect("Quit signal could not be processed");
        });

        let address_manager = AddressManager::new(config.data_dir.as_ref().unwrap(), 4)?;
        let mempool = Mempool::new();
        let blockchain = Blockchain::new(&config.data_dir.as_ref().unwrap());
        #[cfg(feature = "grpc")]
        let blockchain = Arc::new(Mutex::new(blockchain));
        #[cfg(feature = "grpc")]
        let mempool = Arc::new(Mutex::new(mempool));

        #[cfg(feature = "grpc")]
        let (broadcast_channel_tx, broadcast_channel_rx) = {
            let best_block_hash = blockchain
                .lock()
                .await
                .best_block_hash()
                .expect("Blockchain error");
            let best_block = blockchain
                .lock()
                .await
                .get_block(&best_block_hash)
                .expect("Blockchain error");
            watch::channel(BroadcastMessage::BestBlock(best_block.unwrap()))
        };
        #[cfg(feature = "grpc")]
        let rpc_abort = {
            let rpc = RPCNode::new(
                broadcast_channel_rx,
                mempool.clone(),
                blockchain.clone(),
                sender.clone(),
            );
            let addr = format!("{}:{}", "[::1]", config.grpc_port).parse().unwrap();
            let (handle, registration) = AbortHandle::new_pair();
            let rpc_server = Abortable::new(
                tonic::transport::Server::builder()
                    .serve(addr, super::node::server::NodeServer::new(rpc)),
                registration,
            )
            .map_err(|_| ())
            .map(|e| match e {
                Ok(_) => (),
                Err(e) => {
                    warn!("RPC server errored: {:?}", e);
                }
            });
            tokio::spawn(rpc_server);
            debug!("Created RPC server");
            handle
        };

        #[allow(unused_mut)]
        let mut server = Server {
            #[cfg(feature = "grpc")]
            broadcast_channel_tx,
            #[cfg(feature = "grpc")]
            rpc_abort,
            connections: std::collections::HashMap::new(),
            connection_receiver: receiver,
            connection_sender: sender,
            connection_count: 0,
            max_connections_count: config.max_connections,
            utxo_manager: UtxoManager::new(config.data_dir.as_ref().unwrap()),
            blockchain,
            next_id_to_give: 0,
            mempool,
            sync_counter: 3,
            orphan_manager: OrphanBlockManager::new(),
            #[cfg(feature = "matrix_discover")]
            matrix_client: None,
            address_manager,
            origin_port: config.port,
        };
        info!("Node created, listening on port {}", config.port);
        let mut discover_message = "Starting server with: ".to_string();
        #[cfg(feature = "matrix_discover")]
        {
            let mut initial_bots = Vec::new();
            if config.matrix_creds.is_some() {
                match server.start_matrix(&config) {
                    Ok(b) => initial_bots = b,
                    Err(()) => (),
                }
            }
            server.address_manager.set_bots(initial_bots);
            discover_message.push_str(&format!("{} peers from matrix,", initial_bots.len()));
        }
        discover_message.push_str(&format!(
            "{} nodes from address_manager",
            server.address_manager.len()
        ));
        info!("{}", discover_message);
        if let Err(e) = server.main_loop().await {
            match e {
                Error::Quit => (),
                _ => error!("Server failed: {:?}", e),
            }
        }
        Ok(())
    }

    async fn main_loop(mut self) -> Result<(), Error> {
        while let Some(message) = self.connection_receiver.recv().await {
            self.handle_message(message).await?;
        }
        Ok(())
    }

    #[cfg(feature = "matrix_discover")]
    fn start_matrix(&mut self, config: &ServerConfig) -> Result<Vec<String>, ()> {
        let mut initial_bots = Vec::new();
        let matrix_creds = match std::fs::File::open(config.matrix_creds.as_ref().unwrap()) {
            Ok(f) => f,
            Err(e) => {
                warn!("Could not read matrix credentials: {}", e);
                return Err(());
            }
        };
        let matrix_config: crate::bootstrap::matrix::Config =
            match ron::de::from_reader(matrix_creds) {
                Ok(m) => m,
                Err(e) => {
                    warn!("Could not deserialize matrix credentials: {}", e);
                    return Err(());
                }
            };
        let matrix_client = matrix::MatrixClient::new(matrix_config);
        match matrix_client.get_room_id() {
            Ok(Some(room_id)) => {
                if let Err(e) = matrix_client.set_status(&matrix::Status::Online) {
                    warn!("Could not pass online on matrix: {}", e)
                } else {
                    if let Err(e) = matrix_client.set_name(
                        &format!("{}", crate::constants::MAGIC),
                        crate::constants::IP,
                        &format!("{}", config.port),
                    ) {
                        warn!("Could not set matrix displayname: {}", e)
                    } else {
                        match matrix_client
                            .get_bots(&room_id, &format!("{}", crate::constants::MAGIC))
                        {
                            Ok(b) => initial_bots = b,
                            Err(e) => warn!("Could not retrieve initial_peers: {}", e),
                        }
                    }
                }
            }
            Ok(None) => warn!("No room id in matrix response"),
            Err(e) => warn!("Could not get matrix room id: {}", e),
        }
        self.matrix_client = Some(matrix_client);
        Ok(initial_bots)
    }

    async fn broadcast_to_connections(&mut self, message: ServerMessage) -> Result<(), Error> {
        let remotes: Vec<_> = self.connections.keys().copied().collect();
        for remote in remotes {
            self.send(remote, message.clone()).await?;
        }
        Ok(())
    }
    async fn send(&mut self, id: u64, message: ServerMessage) -> Result<(), Error> {
        match self.connections.get_mut(&id) {
            Some(h) => {
                h.send(message).await?;
            }
            None => {
                warn!("Could not send to unkwown connection: {}", id);
            }
        }
        Ok(())
    }

    // The boolean means should execution continue or not
    async fn handle_message(&mut self, message: ConnectionMessage) -> Result<bool, Error> {
        debug!("Server handling {}", message);
        self.address_manager.new_message(&message.source);
        match message.content {
            ConnectionMessageContent::ConnectionFailed(address) => {
                info!("Connection {} failed", address);
                self.address_manager
                    .no_response(crate::data::intern_messages::Peer::from(address));
                if self.connection_count < 10 {
                    self.find_new_peer().await;
                }
            }
            ConnectionMessageContent::Quit => {
                #[cfg(feature = "matrix_discover")]
                {
                    if let Some(matrix_client) = self.matrix_client.take() {
                        info!("Offline in matrix");
                        matrix::async_set_status(matrix_client.config(), &matrix::Status::Offline);
                    }
                }
                #[cfg(feature = "grpc")]
                {
                    info!("Shuting down RPC server");
                    if self
                        .broadcast_channel_tx
                        .send(BroadcastMessage::Quit)
                        .await
                        .is_err()
                    {
                        error!("Cannot stop RPC server")
                    }
                    self.rpc_abort.abort();
                }
                info!("Disconnecting Peers");
                for conn_sender in self.connections.values_mut() {
                    if let Err(e) = conn_sender
                        .send(ServerMessage::Terminate(TerminationReason::Quit))
                        .await
                    {
                        warn!("Could not shutdown connection: {:?}", e)
                    }
                }
                info!("Reseting connection state");
                self.address_manager.reset_state();
                info!("Node shutdown !");
                return Err(Error::Quit);
            }
            ConnectionMessageContent::RetrieveAddr => {
                if let Source::Connection(remote) = message.source {
                    let m = Message::Addr(self.address_manager.get_addr());
                    match self.connections.get_mut(&remote.id) {
                        Some(h) => h.send(ServerMessage::SendMsg(m)).await?,
                        None => warn!("Could not send to {}: unknown connection", remote.id),
                    }
                }
            }
            ConnectionMessageContent::NewAddr(addr) => {
                for address in addr {
                    match tokio::net::TcpStream::connect((
                        std::net::IpAddr::from(address.ip),
                        address.port,
                    ))
                    .timeout(std::time::Duration::from_millis(500))
                    .await
                    {
                        Ok(Ok(_)) => self.address_manager.add_addr(address),
                        Err(_) | Ok(Err(_)) => {
                            warn!(
                                "Recieved invalid address: {:?}:{}",
                                address.ip, address.port
                            );
                        }
                    }
                }
            }
            ConnectionMessageContent::VerifiedAddr(address) => {
                self.address_manager.add_addr(address)
            }
            ConnectionMessageContent::Register(mut sender, host) => {
                if self.connection_count < self.max_connections_count {
                    info!("Registered [{}]", &host.id);
                    self.connections.insert(host.id, sender);
                    self.address_manager.register_addr(host.peer, true);
                    self.connection_count += 1;

                    if self.sync_counter > 0 {
                        self.sync_counter -= 1;
                        let getblocks =
                            Message::GetBlocks(self.blockchain.lock().await.generate_get_blocks()?);
                        let msg = ServerMessage::SendMsg(getblocks);
                        let msg_get_mempool = ServerMessage::SendMsg(Message::GetMempool);
                        self.send(host.id, msg).await?;
                        self.send(host.id, msg_get_mempool).await?;
                    };
                } else {
                    warn!("Too many connections to accept [{}]", &host.id);
                    sender
                        .send(ServerMessage::Terminate(
                            TerminationReason::TooManyConnections,
                        ))
                        .await?;
                }
            }
            ConnectionMessageContent::Clean(host) => {
                if self.connections.remove(&host).is_some() {
                    self.connection_count -= 1;
                };
                if self.connection_count < 10 {
                    self.find_new_peer().await;
                }
                trace!("Cleaned connection [{}]", host);
            }
            ConnectionMessageContent::Disconnect(e, host) => {
                /*if let Error::NoResponse = &e {
                    match host.parse() {
                        Ok(p) => self.address_manager.no_response(p),
                        Err(e) => warn!("Host [{}] is not a socket addr: {:?}", host, e),
                    }
                };
                if self.connections.contains_key(&host) {
                    self.send(
                        host,
                        ServerMessage::Terminate(TerminationReason::RequestedTermination),
                    )
                    .await?;
                }*/
                // TODO what to doo
            }
            ConnectionMessageContent::CheckInv(inv) => {
                let (mut unknown, txs) = self.blockchain.lock().await.get_unknown_blocks(inv)?;
                let (mut unknown_tx, _) = self.mempool.lock().await.get_unknown_tx(txs);
                unknown.append(&mut unknown_tx);
                if !unknown.is_empty() {
                    let get_data = Message::GetData(unknown);
                    if let crate::data::intern_messages::Source::Connection(remote) = message.source
                    {
                        self.send(remote.id, ServerMessage::SendMsg(get_data))
                            .await?;
                    }
                };
            }
            ConnectionMessageContent::Retrieve(get_data) => {
                // GetData
                if let crate::data::intern_messages::Source::Connection(remote) = message.source {
                    let (blocks, remaining) = self.blockchain.lock().await.get_data(get_data)?;
                    for block in blocks {
                        self.send(
                            remote.id,
                            ServerMessage::SendMsg(Message::Block(Box::new(block))),
                        )
                        .await?;
                    }
                    let (txs, _) = self.mempool.lock().await.get_data(remaining);
                    for tx in txs {
                        self.send(remote.id, ServerMessage::SendMsg(Message::Tx(Box::new(tx))))
                            .await?;
                    }
                }
            }
            ConnectionMessageContent::SyncBlocks(get_blocks) => {
                // Handling: Best Block
                let inv = self.blockchain.lock().await.generate_inv(&get_blocks)?;
                if !inv.is_empty() {
                    if let crate::data::intern_messages::Source::Connection(remote) = message.source
                    {
                        self.send(remote.id, ServerMessage::SendMsg(Message::Inv(inv)))
                            .await?;
                    }
                }
            }
            ConnectionMessageContent::Connect(address) => {
                let (ip, port) = (address.ip(), address.port());
                let ip = match ip {
                    std::net::IpAddr::V4(i) => i.to_ipv6_mapped().octets(),
                    std::net::IpAddr::V6(i) => i.octets(),
                };
                let peer = crate::data::intern_messages::Peer { ip, port };
                self.address_manager.register_addr(peer, true);
                let id = self.next_id_to_give;
                self.next_id_to_give += 1;
                if let Err(e) = Connection::initiate(
                    std::net::SocketAddr::from((ip, port)),
                    self.connection_sender.clone(),
                    self.origin_port,
                    id,
                )
                .await
                {
                    warn!(
                        "Could ont initiate connection to {}: {:?}",
                        std::net::SocketAddr::from((ip, port)),
                        e
                    );
                    self.next_id_to_give -= 1;
                };
            }
            ConnectionMessageContent::NewTransaction(tx) => {
                // TODO: Verify tx in mempool insert
                let mut ltx = LinkedTransaction::new(*tx);
                self.utxo_manager.link(&mut ltx);
                self.mempool.lock().await.insert(ltx);
            }
            ConnectionMessageContent::NewBlock(block) => {
                self.handle_new_block(*block, message.source).await?;
            }
            ConnectionMessageContent::NewConnection(socket) => {
                if self.connection_count < self.max_connections_count {
                    trace!("new connection");
                    let id = self.next_id_to_give;
                    self.next_id_to_give += 1;
                    Connection::accept(
                        socket,
                        self.connection_sender.clone(),
                        self.origin_port,
                        id,
                    );
                }
            }
        }
        Ok(true)
    }

    // TODO: Be a good peer finder
    async fn find_new_peer(&mut self) {
        for peer in self.address_manager.get_some_peers(10_usize) {
            let address = std::net::SocketAddr::from((peer.ip, peer.port));
            let id = self.next_id_to_give;
            self.next_id_to_give += 1;
            if let Err(e) = Connection::initiate(
                address.clone(),
                self.connection_sender.clone(),
                self.origin_port,
                id,
            )
            .await
            {
                warn!("New peer {} errored: {:?}", address, e);
                self.next_id_to_give -= 1;
            }
        }
    }

    fn handle_new_block(
        &mut self,
        block: ensicoin_messages::resource::Block,
        source: crate::data::intern_messages::Source,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Error>> + Send + '_>> {
        async move {
            info!("Handling block of height: {}", block.header.height);
            let mut lblock = LinkedBlock::new(block);
            let hash = lblock.header.double_hash();
            self.utxo_manager.link_block(&mut lblock);
            let new_target = self
                .blockchain
                .lock()
                .await
                .get_target_next_block(lblock.header.timestamp)?;
            debug!(
                "Validating block {}",
                ensicoin_serializer::hash_to_string(&lblock.header.double_hash())
            );
            let prev_block = match self
                .blockchain
                .lock()
                .await
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
                let inv = vec![ensicoin_messages::message::InvVect {
                    hash: lblock.header.double_hash(),
                    data_type: ensicoin_messages::message::ResourceType::Block,
                }];
                self.broadcast_to_connections(ServerMessage::SendMsg(Message::Inv(inv)))
                    .await?;
                let addition = self.blockchain.lock().await.new_block(lblock.clone())?;
                match addition {
                    NewAddition::Fork => {
                        info!("Handling fork");
                        self.utxo_manager.register_block(&lblock)?;
                        self.mempool.lock().await.remove_tx(&lblock);
                        let best_block = self.blockchain.lock().await.best_block_hash()?;
                        let common_hash = match self
                            .blockchain
                            .lock()
                            .await
                            .find_common_hash(best_block, hash)?
                        {
                            Some(h) => h,
                            None => return Err(Error::NotFound("merge point".to_string())),
                        };
                        let new_branch = self
                            .blockchain
                            .lock()
                            .await
                            .chain_until(&hash, &common_hash)?;
                        let pop_contex = self.blockchain.lock().await.pop_until(&common_hash)?;
                        for utxo in pop_contex.utxo_to_remove {
                            self.utxo_manager.delete(&utxo)?;
                        }
                        self.utxo_manager.restore(pop_contex.utxo_to_restore)?;
                        for tx in pop_contex.txs_to_restore {
                            let mut ltx = LinkedTransaction::new(tx);
                            self.utxo_manager.link(&mut ltx);
                            self.mempool.lock().await.insert(ltx);
                        }
                        let block_chain =
                            self.blockchain.lock().await.chain_to_blocks(new_branch)?;
                        let mut linked_chain: Vec<_> =
                            block_chain.into_iter().map(LinkedBlock::new).collect();
                        linked_chain
                            .iter_mut()
                            .for_each(|mut lb| self.utxo_manager.link_block(&mut lb));
                        for lb in &linked_chain {
                            self.utxo_manager.register_block(lb)?;
                        }
                        self.blockchain.lock().await.add_chain(linked_chain)?;
                        trace!(
                            "New best block after fork: {}",
                            ensicoin_serializer::hash_to_string(&lblock.header.double_hash())
                        );
                        #[cfg(feature = "grpc")]
                        {
                            if self
                                .broadcast_channel_tx
                                .send(BroadcastMessage::BestBlock(
                                    self.blockchain
                                        .lock()
                                        .await
                                        .get_block(
                                            &self.blockchain.lock().await.best_block_hash()?,
                                        )?
                                        .unwrap(),
                                ))
                                .await
                                .is_err()
                            {
                                error!("Could not broadcast");
                            }
                        }
                    }
                    NewAddition::BestBlock => {
                        trace!(
                            "New best block: {}",
                            ensicoin_serializer::hash_to_string(&lblock.header.double_hash())
                        );
                        self.utxo_manager.register_block(&lblock)?;
                        #[cfg(feature = "grpc")]
                        {
                            if self
                                .broadcast_channel_tx
                                .send(BroadcastMessage::BestBlock(
                                    self.blockchain
                                        .lock()
                                        .await
                                        .get_block(
                                            &self.blockchain.lock().await.best_block_hash()?,
                                        )?
                                        .unwrap(),
                                ))
                                .await
                                .is_err()
                            {
                                error!("Could not broadcast");
                            }
                        }
                    }
                    NewAddition::Nothing => {
                        info!("Added block to a sidechain");
                    }
                }
            } else {
                warn!("Recieved invalid Block from {}", source);
            }
            let best_block_hash = self.blockchain.lock().await.best_block_hash()?;
            let orphan_chain = self.orphan_manager.retrieve_chain(best_block_hash);
            for (s, b) in orphan_chain {
                self.handle_new_block(b, s).await?;
            }
            Ok(())
        }
        .boxed()
    }
}

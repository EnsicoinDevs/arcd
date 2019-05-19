pub mod node {
    include!(concat!(env!("OUT_DIR"), "/ensicoin_rpc.rs"));
}

use node::{
    server, Block, BlockTemplate, ConnectPeerReply, ConnectPeerRequest, DisconnectPeerReply,
    DisconnectPeerRequest, GetBlockByHashReply, GetBlockByHashRequest, GetBlockTemplateReply,
    GetBlockTemplateRequest, GetInfoReply, GetInfoRequest, GetTxByHashReply, GetTxByHashRequest,
    Outpoint, PublishRawBlockReply, PublishRawBlockRequest, PublishRawTxReply, PublishRawTxRequest,
    Tx, TxInput, TxOutput,
};

use crate::{
    constants::{IMPLEMENTATION, VERSION},
    data::intern_messages::{self, BroadcastMessage, ConnectionMessage},
    manager::{Blockchain, Mempool},
};
use ensicoin_serializer::{Deserialize, Deserializer, Serialize, Sha256Result};
use futures::{future, stream, Future, Sink, Stream};
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tokio_bus::Bus;
use tower_grpc::{Request, Response, Streaming};
use tower_hyper::server::{Http, Server};

#[derive(Clone)]
pub struct RPCNode {
    state: Arc<State>,
}

struct State {
    mempool: Arc<RwLock<Mempool>>,
    blockchain: Arc<RwLock<Blockchain>>,
    server_sender: futures::sync::mpsc::Sender<ConnectionMessage>,
    broadcast: Arc<RwLock<Bus<BroadcastMessage>>>,
}

fn tx_to_rpc(tx: ensicoin_messages::resource::Transaction) -> Tx {
    Tx {
        hash: tx.double_hash().to_vec(),
        version: tx.version,
        flags: tx.flags,
        inputs: tx
            .inputs
            .into_iter()
            .map(|input| TxInput {
                script: input.script.serialize().to_vec(),
                previous_output: Some(Outpoint {
                    hash: input.previous_output.hash.to_vec(),
                    index: input.previous_output.index,
                }),
            })
            .collect(),
        outputs: tx
            .outputs
            .into_iter()
            .map(|output| TxOutput {
                value: output.value,
                script: output.script.serialize().to_vec(),
            })
            .collect(),
    }
}

fn block_to_rpc(block: ensicoin_messages::resource::Block) -> Block {
    Block {
        flags: block.header.flags.clone(),
        hash: block.header.double_hash().to_vec(),
        version: block.header.version,
        prev_block: block.header.prev_block.to_vec(),
        merkle_root: block.header.merkle_root.to_vec(),
        timestamp: block.header.timestamp,
        height: block.header.height,
        target: block.header.target.to_vec(),
        txs: block.txs.into_iter().map(|tx| tx_to_rpc(tx)).collect(),
    }
}

impl State {
    fn new(
        broadcast: Arc<RwLock<Bus<BroadcastMessage>>>,
        mempool: Arc<RwLock<Mempool>>,
        blockchain: Arc<RwLock<Blockchain>>,
        server_sender: futures::sync::mpsc::Sender<ConnectionMessage>,
    ) -> State {
        State {
            broadcast,
            mempool,
            blockchain,
            server_sender,
        }
    }
}

impl RPCNode {
    pub fn new(
        broadcast: Arc<RwLock<Bus<BroadcastMessage>>>,
        mempool: Arc<RwLock<Mempool>>,
        blockchain: Arc<RwLock<Blockchain>>,
        sender: futures::sync::mpsc::Sender<ConnectionMessage>,
        bind_address: &str,
        port: u16,
    ) -> Box<Future<Item = (), Error = ()> + Send> {
        let handler = RPCNode {
            state: Arc::new(State::new(broadcast, mempool, blockchain, sender)),
        };
        let new_service = server::NodeServer::new(handler);

        let mut server = Server::new(new_service);
        let http = Http::new().http2_only(true).clone();

        let addr = format!("{}:{}", bind_address, port).parse().unwrap();
        let bind = TcpListener::bind(&addr).unwrap();

        info!("Started gRPC server on port {}", port);

        let serve = bind
            .incoming()
            .for_each(move |sock| {
                if let Err(e) = sock.set_nodelay(true) {
                    return Err(e);
                }
                let serve = server.serve_with(sock, http.clone());
                tokio::spawn(serve.map_err(|e| error!("[gRPC] h2 error: {}", e)));
                Ok(())
            })
            .map_err(|e| error!("[gRPC] accept error: {}", e));
        Box::new(serve)
    }
}

impl node::server::Node for RPCNode {
    type GetInfoFuture = future::FutureResult<Response<GetInfoReply>, tower_grpc::Status>;

    fn get_info(&mut self, _request: Request<GetInfoRequest>) -> Self::GetInfoFuture {
        trace!("[grpc] GetInfo");
        let response = Response::new(GetInfoReply {
            implementation: IMPLEMENTATION.to_string(),
            protocol_version: VERSION,
            best_block_hash: match self.state.blockchain.read().unwrap().best_block_hash() {
                Ok(a) => a.to_vec(),
                Err(_) => Vec::new(),
            },
        });
        future::ok(response)
    }

    type PublishRawTxFuture = future::FutureResult<Response<PublishRawTxReply>, tower_grpc::Status>;

    fn publish_raw_tx(
        &mut self,
        request: Request<PublishRawTxRequest>,
    ) -> Self::PublishRawTxFuture {
        info!("[grpc] PublishRawTx");
        let sender = self.state.server_sender.clone();
        let raw_tx_msg = request.into_inner();

        let mut de = Deserializer::new(bytes::BytesMut::from(raw_tx_msg.raw_tx));
        let tx = match ensicoin_messages::resource::Transaction::deserialize(&mut de) {
            Ok(tx) => tx,
            Err(e) => {
                warn!("[grpc] Error reading tx: {}", e);
                return future::result(Err(tower_grpc::Status::new(
                    tower_grpc::Code::InvalidArgument,
                    format!("Error parsing: {}", e),
                )));
            }
        };
        tokio::spawn(
            sender
                .clone()
                .send(ConnectionMessage::NewTransaction(
                    tx,
                    intern_messages::Source::RPC,
                ))
                .map_err(|e| warn!("[grpc] can't contact server: {}", e))
                .map(|_| ()),
        );
        future::ok(Response::new(PublishRawTxReply {}))
    }

    type PublishRawBlockFuture =
        future::FutureResult<Response<PublishRawBlockReply>, tower_grpc::Status>;

    fn publish_raw_block(
        &mut self,
        request: Request<PublishRawBlockRequest>,
    ) -> Self::PublishRawBlockFuture {
        info!("[grpc] PublishRawBlock");
        let sender = self.state.server_sender.clone();
        let raw_blk_msg = request.into_inner();
        let mut de = Deserializer::new(bytes::BytesMut::from(raw_blk_msg.raw_block));
        let block = match ensicoin_messages::resource::Block::deserialize(&mut de) {
            Ok(b) => b,
            Err(e) => {
                warn!("[grpc] Error reading block: {}", e);
                return future::err(tower_grpc::Status::new(
                    tower_grpc::Code::InvalidArgument,
                    format!("Error parsing: {}", e),
                ));
            }
        };
        tokio::spawn(
            sender
                .clone()
                .send(ConnectionMessage::NewBlock(
                    block,
                    intern_messages::Source::RPC,
                ))
                .map_err(|e| warn!("[grpc] can't contact server: {}", e))
                .map(|_| ()),
        );
        future::ok(Response::new(PublishRawBlockReply {}))
    }

    type GetBlockTemplateStream =
        Box<Stream<Item = GetBlockTemplateReply, Error = tower_grpc::Status> + Send>;
    type GetBlockTemplateFuture =
        future::FutureResult<Response<Self::GetBlockTemplateStream>, tower_grpc::Status>;

    fn get_block_template(
        &mut self,
        request: Request<GetBlockTemplateRequest>,
    ) -> Self::GetBlockTemplateFuture {
        unimplemented!()
    }

    type ConnectPeerFuture = future::FutureResult<Response<ConnectPeerReply>, tower_grpc::Status>;
    fn connect_peer(&mut self, request: Request<ConnectPeerRequest>) -> Self::ConnectPeerFuture {
        let sender = self.state.clone().server_sender.clone();
        let inner = request.into_inner();
        let peer = match inner.peer {
            Some(p) => p,
            None => {
                return future::result(Err(tower_grpc::Status::new(
                    tower_grpc::Code::InvalidArgument,
                    "no peer provided".to_string(),
                )))
            }
        };
        let address = match peer.address {
            Some(a) => a,
            None => {
                return future::result(Err(tower_grpc::Status::new(
                    tower_grpc::Code::InvalidArgument,
                    "no address in peer provided".to_string(),
                )))
            }
        };
        let address = match format!("{}:{}", address.ip, address.port).parse() {
            Ok(a) => a,
            Err(e) => {
                return future::FutureResult::from(Err(tower_grpc::Status::new(
                    tower_grpc::Code::InvalidArgument,
                    format!("{:?}", e),
                )))
            }
        };
        info!("[grpc] Connect to: {}", &address);
        tokio::spawn(
            sender
                .clone()
                .send(ConnectionMessage::Connect(address))
                .map_err(|e| warn!("[grpc] can't contact server: {}", e))
                .map(|_| ()),
        );
        future::ok(tower_grpc::Response::new(ConnectPeerReply {}))
    }
    type DisconnectPeerFuture =
        future::FutureResult<Response<DisconnectPeerReply>, tower_grpc::Status>;
    fn disconnect_peer(
        &mut self,
        request: Request<DisconnectPeerRequest>,
    ) -> Self::DisconnectPeerFuture {
        let sender = self.state.clone().server_sender.clone();
        let inner = request.into_inner();
        let peer = match inner.peer {
            Some(p) => p,
            None => {
                return future::result(Err(tower_grpc::Status::new(
                    tower_grpc::Code::InvalidArgument,
                    "no peer provided".to_string(),
                )))
            }
        };
        let address = match peer.address {
            Some(a) => a,
            None => {
                return future::result(Err(tower_grpc::Status::new(
                    tower_grpc::Code::InvalidArgument,
                    "no address in peer provided".to_string(),
                )))
            }
        };
        let address = match format!("{}:{}", address.ip, address.port).parse() {
            Ok(a) => a,
            Err(e) => {
                return future::FutureResult::from(Err(tower_grpc::Status::new(
                    tower_grpc::Code::InvalidArgument,
                    format!("{:?}", e),
                )))
            }
        };
        info!("[grpc] Disconnect from: {}", &address);
        tokio::spawn(
            sender
                .clone()
                .send(ConnectionMessage::Disconnect(
                    crate::Error::ServerTermination,
                    address,
                ))
                .map_err(|e| warn!("[grpc] can't contact server: {}", e))
                .map(|_| ()),
        );
        future::ok(tower_grpc::Response::new(DisconnectPeerReply {}))
    }

    type GetBlockByHashFuture =
        future::FutureResult<Response<GetBlockByHashReply>, tower_grpc::Status>;
    fn get_block_by_hash(
        &mut self,
        request: Request<GetBlockByHashRequest>,
    ) -> Self::GetBlockByHashFuture {
        let request = request.into_inner();
        let hash = Sha256Result::clone_from_slice(&request.hash);
        let block = match self.state.blockchain.read() {
            Ok(l) => match l.get_block(&hash) {
                Ok(Some(b)) => b,
                Ok(None) => {
                    return future::result(Err(tower_grpc::Status::new(
                        tower_grpc::Code::NotFound,
                        "",
                    )))
                }
                Err(_) => {
                    return future::result(Err(tower_grpc::Status::new(
                        tower_grpc::Code::Internal,
                        "",
                    )))
                }
            },
            Err(_) => {
                return future::result(Err(tower_grpc::Status::new(tower_grpc::Code::Internal, "")))
            }
        };
        future::ok(tower_grpc::Response::new(GetBlockByHashReply {
            block: Some(block_to_rpc(block)),
        }))
    }

    type GetTxByHashFuture = future::FutureResult<Response<GetTxByHashReply>, tower_grpc::Status>;
    fn get_tx_by_hash(&mut self, request: Request<GetTxByHashRequest>) -> Self::GetTxByHashFuture {
        unimplemented!()
    }
}

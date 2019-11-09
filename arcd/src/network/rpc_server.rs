pub mod node {
    tonic::include_proto!("ensicoin_rpc");
}

use crate::utils::big_uint_to_hash;
use node::{
    Block, BlockTemplate, ConnectPeerReply, ConnectPeerRequest, DisconnectPeerReply,
    DisconnectPeerRequest, GetBestBlocksReply, GetBestBlocksRequest, GetBlockByHashReply,
    GetBlockByHashRequest, GetBlockTemplateReply, GetBlockTemplateRequest, GetInfoReply,
    GetInfoRequest, GetTxByHashReply, GetTxByHashRequest, Outpoint, PublishRawBlockReply,
    PublishRawBlockRequest, PublishRawTxReply, PublishRawTxRequest, Tx, TxInput, TxOutput,
};

use crate::{
    constants::{IMPLEMENTATION, VERSION},
    data::intern_messages::{
        BroadcastMessage, ConnectionMessage, ConnectionMessageContent, Source,
    },
    manager::{Blockchain, Mempool},
};
use ensicoin_serializer::{hash_to_string, Deserialize, Deserializer, Sha256Result};
use ensicoin_messages::resource::script::fn_script;
use std::sync::Arc;
use tokio::sync::{mpsc, watch, Mutex};
use tonic::{Request, Response, Status};

fn internal<T, E: std::fmt::Debug>(res: Result<T, E>) -> Result<T, Status> {
    match res {
        Err(e) => {
            warn!("Internal error: {:?}", e);
            Err(Status::new(tonic::Code::Internal, ""))
        }
        Ok(v) => Ok(v),
    }
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
                script: ensicoin_messages::as_bytes(fn_script(&input.script)),
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
                script: ensicoin_messages::as_bytes(fn_script(&output.script)),
            })
            .collect(),
    }
}

fn block_header_to_rpc(header: ensicoin_messages::resource::BlockHeader) -> node::BlockHeader {
    node::BlockHeader {
        flags: header.flags.clone(),
        hash: header.double_hash().to_vec(),
        version: header.version,
        prev_block: header.prev_block.to_vec(),
        merkle_root: header.merkle_root.to_vec(),
        timestamp: header.timestamp,
        height: header.height,
        target: header.target.to_vec(),
        nonce: header.nonce,
    }
}

fn block_to_rpc(block: ensicoin_messages::resource::Block) -> Block {
    Block {
        header: Some(block_header_to_rpc(block.header)),
        txs: block.txs.into_iter().map(tx_to_rpc).collect(),
    }
}

#[derive(Clone)]
pub struct RPCNode {
    mempool: Arc<Mutex<Mempool>>,
    blockchain: Arc<Mutex<Blockchain>>,
    server_sender: mpsc::Sender<ConnectionMessage>,
    broadcast: watch::Receiver<BroadcastMessage>,
}

impl RPCNode {
    pub fn new(
        broadcast: watch::Receiver<BroadcastMessage>,
        mempool: Arc<Mutex<Mempool>>,
        blockchain: Arc<Mutex<Blockchain>>,
        sender: mpsc::Sender<ConnectionMessage>,
    ) -> Self {
        Self {
            mempool,
            blockchain,
            broadcast,
            server_sender: sender,
        }
    }
    async fn produce_block_template(
        mempool: Arc<Mutex<Mempool>>,
        blockchain: Arc<Mutex<Blockchain>>,
        block: &ensicoin_messages::resource::Block,
    ) -> (Vec<node::Tx>, BlockTemplate) {
        let txs: Vec<node::Tx> = mempool
            .lock()
            .await
            .get_tx()
            .into_iter()
            .map(tx_to_rpc)
            .collect();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let height = block.header.height + 1;
        let prev_block = block.header.double_hash();
        let flags = Vec::new();
        let version = crate::constants::VERSION;
        let target = big_uint_to_hash(
            blockchain
                .lock()
                .await
                .get_target_next_block(timestamp)
                .unwrap(),
        );
        (
            txs,
            BlockTemplate {
                timestamp,
                height,
                prev_block: prev_block.to_vec(),
                flags,
                version,
                target: target.to_vec(),
            },
        )
    }
}

type Reply<T> = Result<Response<T>, Status>;

#[tonic::async_trait]
impl node::server::Node for RPCNode {
    async fn get_info(&self, _request: Request<GetInfoRequest>) -> Reply<GetInfoReply> {
        debug!("[grpc] GetInfo");
        let best_block_hash = match self.blockchain.lock().await.best_block_hash() {
            Ok(a) => a.to_vec(),
            Err(_) => Vec::new(),
        };
        let genesis_block_hash = match self.blockchain.lock().await.genesis_hash() {
            Ok(h) => h.to_vec(),
            Err(_) => Vec::new(),
        };
        let response = Response::new(GetInfoReply {
            implementation: IMPLEMENTATION.to_string(),
            protocol_version: VERSION,
            best_block_hash,
            genesis_block_hash,
        });

        Ok(response)
    }

    async fn publish_raw_tx(
        &self,
        request: Request<PublishRawTxRequest>,
    ) -> Reply<PublishRawTxReply> {
        trace!("[grpc] PublishRawTx");
        let raw_tx_msg = request.into_inner();

        let mut de = Deserializer::new(bytes::BytesMut::from(raw_tx_msg.raw_tx));
        let tx = match ensicoin_messages::resource::Transaction::deserialize(&mut de) {
            Ok(tx) => tx,
            Err(e) => {
                warn!("[grpc] Error reading tx: {}", e);
                return Err(tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    format!("Error parsing: {}", e),
                ));
            }
        };
        internal(
            self.server_sender
                .clone()
                .send(ConnectionMessage {
                    content: ConnectionMessageContent::NewTransaction(Box::new(tx)),
                    source: Source::RPC,
                })
                .await,
        )?;
        Ok(Response::new(PublishRawTxReply {}))
    }

    async fn publish_raw_block(
        &self,
        request: Request<PublishRawBlockRequest>,
    ) -> Reply<PublishRawBlockReply> {
        trace!("[grpc] PublishRawBlock");
        let raw_blk_msg = request.into_inner();

        let mut de = Deserializer::new(bytes::BytesMut::from(raw_blk_msg.raw_block));
        let block = match ensicoin_messages::resource::Block::deserialize(&mut de) {
            Ok(b) => b,
            Err(e) => {
                warn!("[grpc] Error reading block: {}", e);
                return Err(tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    format!("Error parsing: {}", e),
                ));
            }
        };
        internal(
            self.server_sender
                .clone()
                .send(ConnectionMessage {
                    content: ConnectionMessageContent::NewBlock(Box::new(block)),
                    source: Source::RPC,
                })
                .await,
        )?;
        Ok(Response::new(PublishRawBlockReply {}))
    }

    type GetBestBlocksStream = mpsc::Receiver<Result<GetBestBlocksReply, Status>>;

    async fn get_best_blocks(
        &self,
        _request: Request<GetBestBlocksRequest>,
    ) -> Reply<Self::GetBestBlocksStream> {
        let (mut out_tx, out_rx) = mpsc::channel(4);
        let mut watch = self.broadcast.clone();

        tokio::spawn(async move {
            while let Some(message) = watch.recv().await {
                let block = match message {
                    BroadcastMessage::BestBlock(block) => block,
                    _ => unreachable!(),
                };
                out_tx
                    .send(Ok(GetBestBlocksReply {
                        hash: block.header.double_hash().to_vec(),
                    }))
                    .await
                    .unwrap();
            }
        });
        Ok(Response::new(out_rx))
    }

    type GetBlockTemplateStream = mpsc::Receiver<Result<GetBlockTemplateReply, Status>>;

    async fn get_block_template(
        &self,
        _request: Request<GetBlockTemplateRequest>,
    ) -> Reply<Self::GetBlockTemplateStream> {
        let mut watch_rx = self.broadcast.clone();
        let best_block_hash = self.blockchain.lock().await.best_block_hash().unwrap();
        let best_block = self
            .blockchain
            .lock()
            .await
            .get_block(&best_block_hash)
            .unwrap()
            .unwrap();
        let (mut out_tx, out_rx) = mpsc::channel(4);
        let mempool = self.mempool.clone();
        let blockchain = self.blockchain.clone();
        let (txs, block_template) =
            RPCNode::produce_block_template(mempool.clone(), blockchain.clone(), &best_block).await;
        out_tx
            .send(Ok(GetBlockTemplateReply {
                txs,
                block_template: Some(block_template),
            }))
            .await
            .unwrap();
        tokio::spawn(async move {
            while let Some(message) = watch_rx.recv().await {
                let block = match message {
                    BroadcastMessage::BestBlock(block) => block,
                    _ => unreachable!(),
                };
                let (txs, block_template) =
                    RPCNode::produce_block_template(mempool.clone(), blockchain.clone(), &block)
                        .await;
                out_tx
                    .send(Ok(GetBlockTemplateReply {
                        txs,
                        block_template: Some(block_template),
                    }))
                    .await
                    .unwrap();
            }
        });
        Ok(Response::new(out_rx))
    }

    async fn connect_peer(&self, request: Request<ConnectPeerRequest>) -> Reply<ConnectPeerReply> {
        let inner = request.into_inner();
        let peer = match inner.peer {
            Some(p) => p,
            None => {
                return Err(tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    "no peer provided".to_string(),
                ))
            }
        };
        let address = match peer.address {
            Some(a) => a,
            None => {
                return Err(tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    "no address in peer provided".to_string(),
                ))
            }
        };
        let address = match format!("{}:{}", address.ip, address.port).parse() {
            Ok(a) => a,
            Err(e) => {
                return Err(tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    format!("{:?}", e),
                ))
            }
        };
        info!("[grpc] Connect to: {}", &address);
        internal(
            self.server_sender
                .clone()
                .send(ConnectionMessage {
                    content: ConnectionMessageContent::Connect(address),
                    source: Source::RPC,
                })
                .await,
        )?;
        Ok(tonic::Response::new(ConnectPeerReply {}))
    }
    async fn disconnect_peer(
        &self,
        request: Request<DisconnectPeerRequest>,
    ) -> Reply<DisconnectPeerReply> {
        let inner = request.into_inner();
        let peer = match inner.peer {
            Some(p) => p,
            None => {
                return Err(tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    "no peer provided".to_string(),
                ))
            }
        };
        let address = match peer.address {
            Some(a) => a,
            None => {
                return Err(tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    "no address in peer provided".to_string(),
                ))
            }
        };
        let address = match format!("{}:{}", address.ip, address.port).parse() {
            Ok(a) => a,
            Err(e) => {
                return Err(tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    format!("{:?}", e),
                ))
            }
        };
        info!("[grpc] Disconnect from: {}", &address);
        internal(
            self.server_sender
                .clone()
                .send(ConnectionMessage {
                    content: ConnectionMessageContent::Disconnect(
                        crate::Error::ServerTermination,
                        address,
                    ),
                    source: Source::RPC,
                })
                .await,
        )?;
        Ok(tonic::Response::new(DisconnectPeerReply {}))
    }

    async fn get_block_by_hash(
        &self,
        request: Request<GetBlockByHashRequest>,
    ) -> Reply<GetBlockByHashReply> {
        let request = request.into_inner();
        if request.hash.len() != 32 {
            return Err(tonic::Status::new(
                tonic::Code::InvalidArgument,
                "hash is not 32 bytes",
            ));
        };
        let hash = Sha256Result::clone_from_slice(&request.hash);
        let block = match self.blockchain.lock().await.get_block(&hash) {
            Ok(Some(b)) => b,
            Ok(None) => return Err(tonic::Status::new(tonic::Code::NotFound, "")),
            Err(_) => return Err(tonic::Status::new(tonic::Code::Internal, "")),
        };
        Ok(Response::new(GetBlockByHashReply {
            block: Some(block_to_rpc(block)),
            main_chain: true,
        }))
    }
    async fn get_block_header_by_hash(
        &self,
        request: Request<node::GetBlockHeaderByHashRequest>,
    ) -> Reply<node::GetBlockHeaderByHashReply> {
        let request = request.into_inner();
        if request.hash.len() != 32 {
            return Err(tonic::Status::new(
                tonic::Code::InvalidArgument,
                "hash is not 32 bytes",
            ));
        };
        let hash = Sha256Result::clone_from_slice(&request.hash);
        let block = match self.blockchain.lock().await.get_block(&hash) {
            Ok(Some(b)) => b,
            Ok(None) => return Err(tonic::Status::new(tonic::Code::NotFound, "")),
            Err(_) => return Err(tonic::Status::new(tonic::Code::Internal, "")),
        };
        Ok(Response::new(node::GetBlockHeaderByHashReply {
            header: Some(block_header_to_rpc(block.header)),
            main_chain: true,
        }))
    }

    async fn get_tx_by_hash(
        &self,
        request: Request<GetTxByHashRequest>,
    ) -> Reply<GetTxByHashReply> {
        let request = request.into_inner();
        if request.hash.len() != 32 {
            return Err(tonic::Status::new(
                tonic::Code::InvalidArgument,
                "hash is not 32 bytes",
            ));
        };
        let hash = Sha256Result::clone_from_slice(&request.hash);
        let tx = match self.mempool.lock().await.get_tx_by_hash(&hash) {
            Some(tx) => tx,
            None => {
                return Err(tonic::Status::new(
                    tonic::Code::NotFound,
                    hash_to_string(&hash),
                ))
            }
        };
        Ok(Response::new(GetTxByHashReply {
            tx: Some(tx_to_rpc(tx)),
        }))
    }

    type GetNewTxStream = mpsc::Receiver<Result<node::GetNewTxReply, Status>>;
}

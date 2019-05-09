pub mod node {
    include!(concat!(env!("OUT_DIR"), "/ensicoin_rpc.node.rs"));
}

use node::{
    server, GetBestBlockHashReply, GetBestBlockHashRequest, GetBlockByHashReply,
    GetBlockByHashRequest, GetBlockTemplateReply, GetBlockTemplateRequest, GetInfoReply,
    GetInfoRequest, GetTxByHashReply, GetTxByHashRequest, PublishRawBlockReply,
    PublishRawBlockRequest, PublishRawTxReply, PublishRawTxRequest,
};

#[derive(Debug, Clone)]
pub struct RPCNode;

impl node::server::Node for RPCNode {}

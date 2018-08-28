#![allow(unused)] // TODO: Get rid of this when RPC is implemented
use proto::types::rpc::RPCExecutionBody;

#[derive(ClientMessage, new)]
#[packet_id = 0x20]
pub struct RPCExecute {
    rpc_definition_id: u16,
    unique_id: u16,
    body: RPCExecutionBody,
}

pub struct RPCResponse {
    rpc_definition_id: u16,
    unique_id: u16,

}

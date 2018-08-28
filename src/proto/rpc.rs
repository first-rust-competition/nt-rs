use nt_packet::{ClientMessage, ServerMessage};
use proto::types::rpc::{Parameter, RPCExecutionBody};
use leb128::LEB128Write;
use nt::state::State;

use std::sync::{Arc, Mutex};
use failure;

use bytes::{BytesMut, BufMut, Buf};

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

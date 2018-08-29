use proto::types::rpc::*;
use proto::types::EntryValue;

use bytes::Buf;
use nt_packet::BufExt;
use nt::state::State;

use std::sync::{Arc, Mutex};

#[derive(ClientMessage, new)]
#[packet_id = 0x20]
pub struct RPCExecute {
    rpc_definition_id: u16,
    unique_id: u16,
    body: RPCExecutionBody,
}

#[derive(Debug)]
pub struct RPCResponse {
    pub rpc_definition_id: u16,
    pub unique_id: u16,
    pub body: RPCResponseBody,
}

impl RPCResponse {
    pub fn decode(mut buf: &mut Buf, state: &Arc<Mutex<State>>) -> Result<(Self, usize), ::failure::Error> {
        let mut bytes_read = 0;

        let rpc_definition_id = buf.read_u16_be()?;
        let unique_id = buf.read_u16_be()?;

        bytes_read += 4;

        let data = {
            // Can't bind because of !Send for MutexGuard
            let state = state.lock().unwrap();
            state.entries()[&rpc_definition_id].clone()
        };

        let rpc = match data.value {
            EntryValue::RPCDef(def) => def,
            _ => bail!("Invalid RPC entry id {}. Entry is {:?}", rpc_definition_id, data.value.entry_type())
        };

        let (body, bytes) = RPCResponseBody::decode(buf, rpc)?;
        bytes_read += bytes;

        Ok((RPCResponse { rpc_definition_id, unique_id, body }, bytes_read))
    }
}
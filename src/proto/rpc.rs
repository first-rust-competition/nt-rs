use nt_packet::ClientMessage;
use proto::types::rpc::*;
use leb128::LEB128Write;

use bytes::{BytesMut, BufMut};

#[allow(unused)]
pub struct RPCExecute {
    rpc_definition_id: u16,
    unique_id: u16,
    parameters: Vec<Parameter>
}

//pub struct RPCResponse {
//    rpc_definition_id: u16,
//    unique_id: u16,
//
//}

impl ClientMessage for RPCExecute {
    fn encode(&self, buf: &mut BytesMut) {
        buf.put_u8(0x20);
        buf.put_u16_be(self.rpc_definition_id);
        buf.put_u16_be(self.unique_id);

        buf.write_unsigned(self.parameters.len() as u64).unwrap();
        for param in &self.parameters {
            param.encode(buf);
        }
    }
}

//impl ServerMessage for RPCExecute {
//    fn decode(buf: &mut Buf) -> (Option<Self>, usize) where Self: Sized {
//        let mut bytes_read = 0;
//        let rpc_definition_id = buf.get_u16_be();
//        bytes_read += 2;
//        let unique_id = buf.get_u16_be();
//        bytes_read += 2;
//
//        let len = ::proto::types::leb128::read(buf);
//        bytes_read += 1;
//        let mut parameters = Vec::with_capacity(len);
//
//        for i in 0..len {
//            let (param, bytes) = Parameter::decode(buf);
//            parameters[i] = param.unwrap();
//            bytes_read += bytes;
//        }
//
//        (Some(RPCExecute { rpc_definition_id, unique_id, parameters }), bytes_read)
//    }
//}
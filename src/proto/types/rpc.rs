#![allow(missing_docs)] //TODO: Get rid of this when I can
use super::*;
use nt::state::State;

use std::sync::{Arc, Mutex};

use bytes::{Bytes, Buf, IntoBuf};
use leb128::LEB128Read;

pub enum RPCExecutionBody {
    V1(RPCV1ExecuteBody),
    V0(RPCV0ExecuteBody),
}

#[derive(Debug)]
pub enum RPCResponseBody {
    V1(RPCV1ResponseBody),
    V0(RPCV0ResponseBody),
}

impl ClientMessage for RPCExecutionBody {
    fn encode(&self, buf: &mut BytesMut) {
        match *self {
            RPCExecutionBody::V0(ref v0) => v0.encode(buf),
            RPCExecutionBody::V1(ref v1) => v1.encode(buf),
        }
    }
}

impl RPCResponseBody {
    //noinspection ALL (typechecking broken in intellij-rust for custom derives. Disable to get rid of annoying errors)
    pub fn decode(mut buf: &mut Buf, rpc: RPCDefinitionData) -> Result<(Self, usize), ::failure::Error> {
        match rpc.version {
            0x00 => RPCV0ResponseBody::decode(buf).map(|(body, bytes)| (RPCResponseBody::V0(body), bytes)),
            0x01 => RPCV1ResponseBody::decode(buf, rpc).map(|(body, bytes)| (RPCResponseBody::V1(body), bytes)),
            _ => bail!("Invalid RPC version: {}", rpc.version),
        }
    }
}

#[derive(ServerMessage, Debug)]
pub struct RPCV0ResponseBody {
    pub bytes: Vec<u8>
}

#[derive(Debug)]
pub struct RPCV1ResponseBody {
    pub values: Vec<EntryValue>
}

impl RPCV1ResponseBody {
    fn decode(mut buf: &mut Buf, rpc: RPCDefinitionData) -> Result<(Self, usize), ::failure::Error> {
        let mut bytes_read = 0;
        {
            let (l, bytes) = buf.read_unsigned()?;
            bytes_read += bytes;
        }

        let mut results = Vec::with_capacity(rpc.result_size);

        for i in 0..rpc.result_size {
            let mut result = &rpc.results[i];
            let (res, bytes) = result.result_type.get_entry(buf)?;
            bytes_read += bytes;
            results.push(res);
        }

        Ok((RPCV1ResponseBody { values: results }, bytes_read))
    }
}

#[derive(ClientMessage, new)]
pub struct RPCV1ExecuteBody {
    parameters: Vec<Parameter>,
}

#[derive(ClientMessage, new)]
pub struct RPCV0ExecuteBody {
    bytes: Vec<u8>
}

#[derive(Debug, Clone, PartialEq)]
pub struct RPCDefinitionData {
    pub version: u8,
    pub procedure_name: String,
    pub parameters_size: usize,
    pub parameters: Vec<Parameter>,
    pub result_size: usize,
    pub results: Vec<RpcResult>,
}

#[derive(Debug, Clone, PartialEq, ClientMessage)]
pub struct Parameter {
    parameter_type: EntryType,
    parameter_name: String,
    parameter_default: EntryValue,
}

#[derive(Debug, Clone, PartialEq, ServerMessage, ClientMessage)]
pub struct RpcResult {
    result_type: EntryType,
    result_name: String,
}

impl ServerMessage for Parameter {
    fn decode(buf: &mut Buf) -> Result<(Self, usize), ::failure::Error> {
        let mut bytes_read = 0;
        let parameter_type = {
            let (parameter_type, bytes) = EntryType::decode(buf)?;
            bytes_read += bytes;
            parameter_type
        };

        let parameter_name = {
            let (parameter_name, bytes) = String::decode(buf)?;
            bytes_read += bytes;
            parameter_name
        };

        let (parameter_default, bytes_read_entry) = parameter_type.get_entry(buf)?;
        bytes_read += bytes_read_entry;

        Ok((Parameter {
            parameter_type,
            parameter_name,
            parameter_default,
        }, bytes_read))
    }
}

impl ServerMessage for RPCDefinitionData {
    fn decode(mut buf: &mut Buf) -> ::std::result::Result<(Self, usize), ::failure::Error> {
        let mut bytes_read = 0;

        let mut buf = {
            let (len, bytes) = buf.read_unsigned()?;

            let len = len as usize;

            bytes_read += bytes;

            let mut slice = vec![0u8; len];
            buf.copy_to_slice(&mut slice[..]);
            Bytes::from(slice).into_buf()
        };

        let ver = buf.read_u8()?; // RPC Version
        bytes_read += 1;

        if ver == 0x00 {
            return Ok((RPCDefinitionData {
                version: ver,
                procedure_name: "".to_string(),
                result_size: 0,
                parameters_size: 0,
                results: Vec::new(),
                parameters: Vec::new(),
            }, bytes_read));
        }

        let name = {
            let (s, bytes) = String::decode(&mut buf)?;
            bytes_read += bytes;
            s
        };

        let num_params = buf.read_u8()? as usize;
        bytes_read += 1;

        let mut params = Vec::with_capacity(num_params);

        for _ in 0..num_params {
            let (param, size) = Parameter::decode(&mut buf)?;
            bytes_read += size;
            params.push(param);
        }


        let num_outputs = buf.read_u8()? as usize;
        bytes_read += 1;

        let mut outputs = Vec::with_capacity(num_outputs);

        for _ in 0..num_outputs {
            let (output, size) = RpcResult::decode(&mut buf)?;
            bytes_read += size;
            outputs.push(output);
        }

        Ok((RPCDefinitionData {
            version: 0x01,
            procedure_name: name,
            parameters_size: num_params,
            parameters: params,
            result_size: num_outputs,
            results: outputs,
        }, bytes_read))
    }
}
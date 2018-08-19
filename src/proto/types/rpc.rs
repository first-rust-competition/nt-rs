use super::*;

use bytes::{Bytes, Buf, IntoBuf};
use leb128::LEB128Read;

#[derive(Debug, Clone, PartialEq)]
pub struct RPCDefinitionData {
    version: u8,
    procedure_name: String,
    parameters_size: usize,
    parameters: Vec<Parameter>,
    result_size: usize,
    results: Vec<Result>,
}

#[derive(Debug, Clone, PartialEq, ClientMessage)]
pub struct Parameter {
    parameter_type: EntryType,
    parameter_name: String,
    parameter_default: EntryValue,
}

#[derive(Debug, Clone, PartialEq, ServerMessage, ClientMessage)]
pub struct Result {
    result_type: EntryType,
    result_name: String,
}

impl ServerMessage for Parameter {
    fn decode(buf: &mut Buf) -> (Option<Self>, usize) {
        let mut bytes_read = 0;
        let parameter_type = {
            let (parameter_type, bytes) = EntryType::decode(buf);
            bytes_read += bytes;
            parameter_type.unwrap()
        };

        let parameter_name = {
            let (parameter_name, bytes) = String::decode(buf);
            bytes_read += bytes;
            parameter_name.unwrap()
        };

        let (parameter_default, bytes_read_entry) = parameter_type.get_entry(buf);
        bytes_read += bytes_read_entry;

        (Some(Parameter {
            parameter_type,
            parameter_name,
            parameter_default,
        }), bytes_read)
    }
}

impl ServerMessage for RPCDefinitionData {
    fn decode(mut buf: &mut Buf) -> (Option<Self>, usize) {
        let mut bytes_read = 0;

        let mut buf = {
            let (len, bytes) = buf.read_unsigned().unwrap();
            if len == 1 {
                return (None, bytes);
            }

            let len = len as usize;

            bytes_read += bytes;

            let mut slice = vec![0u8; len];
            buf.copy_to_slice(&mut slice[..]);
            Bytes::from(slice).into_buf()
        };

        buf.get_u8(); // Version; guaranteed to be 0x01
        bytes_read += 1;

        let name = {
            let (s, bytes) = String::decode(&mut buf);
            bytes_read += bytes;
            s.unwrap()
        };

        let num_params = buf.get_u8() as usize;
        bytes_read += 1;

        let mut params = Vec::with_capacity(num_params);

        for _ in 0..num_params {
            let (param, size) = Parameter::decode(&mut buf);
            bytes_read += size;
            params.push(param.unwrap());
        }


        let num_outputs = buf.get_u8() as usize;
        bytes_read += 1;

        let mut outputs = Vec::with_capacity(num_outputs);

        for _ in 0..num_outputs {
            let (output, size) = Result::decode(&mut buf);
            bytes_read += size;
            outputs.push(output.unwrap());
        }

        (Some(RPCDefinitionData {
            version: 0x01,
            procedure_name: name,
            parameters_size: num_params,
            parameters: params,
            result_size: num_outputs,
            results: outputs,
        }), bytes_read)
    }
}
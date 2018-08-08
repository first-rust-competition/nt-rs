use super::*;

#[derive(Debug)]
pub struct RPCDefinitionData {
    version: u8,
    procedure_name: String,
    parameters_size: usize,
    parameters: Vec<Parameter>
}

#[derive(Debug)]
pub struct Parameter {
    parameter_type: EntryType,
    parameter_name: String,
    parameter_default: EntryValue,
    result_size: usize,
    results: Vec<Result>
}

#[derive(Debug)]
pub struct Result {
    result_type: EntryType,
    result_name: String,
}

impl ServerMessage for Result {
    fn decode(buf: &mut Buf) -> (Option<Self>, usize) {
        let (result_type, bytes_read) = EntryType::decode(buf);
        let (result_name, name_bytes_read) = String::decode(buf);

        (Some(Result { result_type: result_type.unwrap(), result_name: result_name.unwrap() }), bytes_read + name_bytes_read)
    }
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


        let len = buf.get_u8() as usize;
        bytes_read += 1;

        let mut results = Vec::with_capacity(len);
        for i in 0..len {
            let (res, bytes) = Result::decode(buf);
            bytes_read += bytes;
            results[i] = res.unwrap();
        }

        (Some(Parameter {
            parameter_type,
            parameter_name,
            parameter_default,
            result_size: len,
            results
        }), bytes_read)
    }
}

impl ServerMessage for RPCDefinitionData {
    fn decode(buf: &mut Buf) -> (Option<Self>, usize) {
        let mut bytes_read = 0;

        let ver = buf.get_u8();
        bytes_read += 1;
        let procedure_name = {
            let (procedure_name, string_bytes_read) = String::decode(buf);
            bytes_read += string_bytes_read;
            procedure_name.unwrap()
        };

        let len = buf.get_u8() as usize;
        bytes_read += 1;
        let mut parameters = Vec::with_capacity(len);

        for i in 0..len {
            let (param, bytes) = Parameter::decode(buf);
            bytes_read += bytes;
            parameters[i] = param.unwrap();
        }

        (Some(RPCDefinitionData { version: ver, procedure_name, parameters_size: len, parameters }), bytes_read)
    }
}
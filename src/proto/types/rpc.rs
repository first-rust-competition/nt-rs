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
    fn decode(buf: &mut Buf) -> Option<Self> {
        let result_type = EntryType::decode(buf)?;
        let result_name = String::decode(buf)?;

        Some(Result { result_type, result_name })
    }
}

impl ServerMessage for Parameter {
    fn decode(buf: &mut Buf) -> Option<Self> {
        let parameter_type = EntryType::decode(buf)?;
        let parameter_name = String::decode(buf)?;
        let parameter_default = parameter_type.get_entry(buf);

        let len = buf.get_u8() as usize;
        let results = Vec::with_capacity(len);
        for i in 0..len {
            results[i] = Result::decode(buf)?;
        }

        Some(Parameter { parameter_type, parameter_name, parameter_default, result_size: len, results})
    }
}

impl ServerMessage for RPCDefinitionData {
    fn decode(buf: &mut Buf) -> Option<Self> {
        let ver = buf.get_u8();
        let procedure_name = String::decode(buf)?;
        let len = buf.get_u8() as usize;
        let parameters = Vec::with_capacity(len);
        for i in 0..len {
            parameters[i] = Parameter::decode(buf)?;
        }

        Some(RPCDefinitionData { version: ver, procedure_name, parameters_size: len, parameters })
    }
}
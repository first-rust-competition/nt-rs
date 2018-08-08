use nt_packet::ServerMessage;
use bytes::Buf;
use self::rpc::RPCDefinitionData;

mod rpc;


#[derive(Debug)]
pub enum EntryType {
    Boolean,
    Double,
    String,
    RawData,
    BooleanArray,
    DoubleArray,
    StringArray,
    RPCDef,
}

#[derive(Debug)]
pub enum EntryValue {
    Boolean(bool),
    Double(f64),
    String(String),
    RawData(Vec<u8>),
    BooleanArray(Vec<bool>),
    DoubleArray(Vec<f64>),
    StringArray(Vec<String>),
    RPCDef(RPCDefinitionData),
}

impl ServerMessage for EntryType {
    fn decode(buf: &mut Buf) -> Option<Self> {
        let byte = buf.get_u8();
        match byte {
            0x00 => Some(EntryType::Boolean),
            0x01 => Some(EntryType::Double),
            0x02 => Some(EntryType::String),
            0x03 => Some(EntryType::RawData),
            0x10 => Some(EntryType::BooleanArray),
            0x11 => Some(EntryType::DoubleArray),
            0x12 => Some(EntryType::StringArray),
            0x20 => Some(EntryType::RPCDef),
            _ => None
        }
    }
}

impl EntryType {
    pub fn get_entry(&self, buf: &mut Buf) -> EntryValue {
        match self {
            &EntryType::Boolean => EntryValue::Boolean(buf.get_u8() == 1),
            &EntryType::Double => EntryValue::Double(buf.get_f64_be()),
            &EntryType::String => EntryValue::String(String::decode(buf).unwrap()),
            &EntryType::RawData => {
                let len = ::leb128::read(buf) as usize;
                let mut data = vec![0u8; len];
                buf.copy_to_slice(&mut data[..]);
                EntryValue::RawData(data)
            }
            &EntryType::BooleanArray => {
                let len = buf.get_u8() as usize;
                let arr = vec![false; len];

                for i in 0..len {
                    let byte = buf.get_u8();
                    arr[i] = byte == 1;
                }

                EntryValue::BooleanArray(arr)
            }
            &EntryType::DoubleArray => {
                let len = buf.get_u8() as usize;
                let arr = vec![0f64; len];

                for i in 0..len {
                    let val = buf.get_f64_be();
                    arr[i] = val;
                }

                EntryValue::DoubleArray(arr)
            }
            &EntryType::StringArray => {
                let len = buf.get_u8() as usize;
                let arr = vec!["".to_string(); len];

                for i in 0..len {
                    let val = String::decode(buf).unwrap();
                    arr[i] = val;
                }

                EntryValue::StringArray(arr)
            }
            &EntryType::RPCDef => EntryValue::RPCDef(RPCDefinitionData::decode(buf).unwrap())
        }
    }
}
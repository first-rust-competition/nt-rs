use nt_packet::ServerMessage;
use bytes::Buf;
use self::rpc::RPCDefinitionData;

mod rpc;
mod leb128;


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
    fn decode(buf: &mut Buf) -> (Option<Self>, usize) {
        let byte = buf.get_u8();
        let this = match byte {
            0x00 => Some(EntryType::Boolean),
            0x01 => Some(EntryType::Double),
            0x02 => Some(EntryType::String),
            0x03 => Some(EntryType::RawData),
            0x10 => Some(EntryType::BooleanArray),
            0x11 => Some(EntryType::DoubleArray),
            0x12 => Some(EntryType::StringArray),
            0x20 => Some(EntryType::RPCDef),
            _ => None
        };
        (this, 1)
    }
}

impl EntryType {
    pub fn get_entry(&self, buf: &mut Buf) -> (EntryValue, usize) {
        match self {
            &EntryType::Boolean => (EntryValue::Boolean(buf.get_u8() == 1), 1),
            &EntryType::Double => (EntryValue::Double(buf.get_f64_be()), 8),
            &EntryType::String => {
                let (value, bytes_read) = String::decode(buf);
                (EntryValue::String(value.unwrap()), bytes_read)
            }
            &EntryType::RawData => {
                let len = leb128::read(buf) as usize;
                let mut data = vec![0u8; len];
                buf.copy_to_slice(&mut data[..]);
                (EntryValue::RawData(data), len + 1)
            }
            &EntryType::BooleanArray => {
                let mut bytes_read = 0;
                let len = buf.get_u8() as usize;
                bytes_read += 1;
                let mut arr = vec![false; len];

                for i in 0..len {
                    let byte = buf.get_u8();
                    bytes_read += 1;
                    arr[i] = byte == 1;
                }

                (EntryValue::BooleanArray(arr), bytes_read)
            }
            &EntryType::DoubleArray => {
                let mut bytes_read = 0;
                let len = buf.get_u8() as usize;
                bytes_read += 1;
                let mut arr = vec![0f64; len];

                for i in 0..len {
                    let val = buf.get_f64_be();
                    bytes_read += 8;
                    arr[i] = val;
                }

                (EntryValue::DoubleArray(arr), bytes_read)
            }
            &EntryType::StringArray => {
                let mut bytes_read = 0;

                let len = buf.get_u8() as usize;
                bytes_read += 1;
                let mut arr = Vec::with_capacity(len);

                for i in 0..len {
                    let (val, bytes) = String::decode(buf);
                    bytes_read += bytes;
                    arr[i] = val.unwrap();
                }

                (EntryValue::StringArray(arr), bytes_read)
            }
            &EntryType::RPCDef => {
                let (def, bytes) = RPCDefinitionData::decode(buf);
                (EntryValue::RPCDef(def.unwrap()), bytes)
            }
        }
    }
}
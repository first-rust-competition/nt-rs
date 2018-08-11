use self::rpc::RPCDefinitionData;
use nt_packet::*;
use bytes::{Buf, BufMut, BytesMut};

use std::convert::AsRef;

mod rpc;
mod leb128;

pub struct EntryData {
    pub name: String,
    pub flags: u8,
    pub value: EntryValue
}

impl EntryData {
    pub fn new(name: String, flags: u8, value: EntryValue) -> EntryData {
        EntryData {
            name,
            flags,
            value
        }
    }
}

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

impl ClientMessage for EntryType {
    fn encode(&self, buf: &mut BytesMut) {
        let byte = match self {
            &EntryType::Boolean => 0x00,
            &EntryType::Double => 0x01,
            &EntryType::String => 0x02,
            &EntryType::RawData => 0x03,
            &EntryType::BooleanArray => 0x10,
            &EntryType::DoubleArray => 0x11,
            &EntryType::StringArray => 0x12,
            _ => panic!()
        };

        buf.put_u8(byte);
    }
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

impl ClientMessage for EntryValue {
    fn encode(&self, buf: &mut BytesMut) {
        match &self {
            &EntryValue::Boolean(val) => buf.put_u8(*val as u8),
            &EntryValue::Double(val) => buf.put_f64_be(*val),
            &EntryValue::String(val) => val.encode(buf),
            &EntryValue::RawData(val) => {
                leb128::write(buf, val.len() as u64);
                buf.put_slice(&val[..]);
            }
            &EntryValue::BooleanArray(val) => {
                leb128::write(buf, val.len() as u64);

                for b in val {
                    buf.put_u8(*b as u8)
                }
            }
            &EntryValue::DoubleArray(val) => {
                leb128::write(buf, val.len() as u64);

                for d in val {
                    buf.put_f64_be(*d);
                }
            }
            &EntryValue::StringArray(val) => {
                leb128::write(buf, val.len() as u64);

                for s in val {
                    s.encode(buf);
                }
            }
            _ => panic!()
        }
    }
}

impl EntryValue {
    pub fn entry_type(&self) -> EntryType {
        match self {
            EntryValue::Boolean(_) => EntryType::Boolean,
            EntryValue::Double(_) => EntryType::Double,
            EntryValue::String(_) => EntryType::String,
            EntryValue::RawData(_) => EntryType::RawData,
            EntryValue::BooleanArray(_) => EntryType::BooleanArray,
            EntryValue::DoubleArray(_) => EntryType::DoubleArray,
            EntryValue::StringArray(_) => EntryType::StringArray,
            EntryValue::RPCDef(_) => EntryType::RPCDef
        }
    }
}

impl<T> From<T> for EntryType
    where T: AsRef<str>
{
    fn from(val: T) -> Self {
        info!("{}", val.as_ref());
        match val.as_ref().to_lowercase().as_str() {
            "string" => EntryType::String,
            "bool" => EntryType::Boolean,
            "double" => EntryType::Double,
            _ => unimplemented!()
        }
    }
}
use self::rpc::RPCDefinitionData;
use nt_packet::*;
use bytes::{Buf, BufMut, BytesMut};
use leb128::{LEB128Read, LEB128Write};

use std::convert::AsRef;

pub mod rpc;

/// Struct containing the data associated with an entry.
/// Used interally to store entries
#[derive(Clone, Debug, PartialEq)]
pub struct EntryData {
    /// The name associated with this entry
    pub name: String,
    /// The flags associated with this entry
    pub flags: u8,
    /// The value associated with this entry
    pub value: EntryValue,
    /// The most recent sequence number associated with this entry
    pub seqnum: u16,
}

impl EntryData {
    /// Creates a new [`EntryData`] with the given parameters, and a sequence number of 1
    pub fn new(name: String, flags: u8, value: EntryValue) -> EntryData {
        Self::new_with_seqnum(name, flags, value, 1)
    }

    #[doc(hidden)]
    pub(crate) fn new_with_seqnum(name: String, flags: u8, value: EntryValue, seqnum: u16) -> EntryData {
        EntryData {
            name,
            flags,
            value,
            seqnum,
        }
    }
}

/// Corresponds to the type tag that NetworkTables presents prior to the corresponding [`EntryValue`]
#[derive(Debug, PartialEq, Clone)]
pub enum EntryType {
    /// Represents a boolean entry value
    Boolean,
    /// Represents a double entry value
    Double,
    /// Represents a string entry value
    String,
    /// Represents a raw data entry value
    RawData,
    /// Represents a boolean array entry value
    BooleanArray,
    /// Represents a double array entry value
    DoubleArray,
    /// Represents a string array entry value
    StringArray,
    /// Represents an RPC definition entry value
    RPCDef,
}

/// Enum representing the different values that NetworkTables supports
#[derive(Debug, Clone, PartialEq)]
pub enum EntryValue {
    /// An entry value containing a single boolean
    Boolean(bool),
    /// An entry value containing a single double
    Double(f64),
    /// An entry value containing a single string
    String(String),
    /// An entry value containing raw data
    RawData(Vec<u8>),
    /// An entry value containing a boolean array
    BooleanArray(Vec<bool>),
    /// An entry value containing a double array
    DoubleArray(Vec<f64>),
    /// An entry value containing a string array
    StringArray(Vec<String>),
    /// An entry value containing definition data for a RPC
    RPCDef(RPCDefinitionData),
    /// Dummy entry value marking malformed RPC definition data
    Pass,
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
    fn decode(mut buf: &mut Buf) -> Result<(Self, usize), ::failure::Error> {
        let byte = buf.read_u8()?;
        debug!("Got entry tag {}", byte);
        let this = match byte {
            0x00 => EntryType::Boolean,
            0x01 => EntryType::Double,
            0x02 => EntryType::String,
            0x03 => EntryType::RawData,
            0x10 => EntryType::BooleanArray,
            0x11 => EntryType::DoubleArray,
            0x12 => EntryType::StringArray,
            0x20 => EntryType::RPCDef,
            _ => bail!("Invalid EntryType tag")
        };
        Ok((this, 1))
    }
}

impl EntryType {
    /// Deserializes an [`EntryValue`] of type `self` from the given `buf`
    pub fn get_entry(&self, mut buf: &mut Buf) -> Result<(EntryValue, usize), ::failure::Error> {
        match self {
            &EntryType::Boolean => Ok((EntryValue::Boolean(buf.read_u8()? == 1), 1)),
            &EntryType::Double => Ok((EntryValue::Double(buf.read_f64_be()?), 8)),
            &EntryType::String => {
                let (value, bytes_read) = String::decode(buf)?;
                Ok((EntryValue::String(value), bytes_read))
            }
            &EntryType::RawData => {
                let (len, size) = buf.read_unsigned().unwrap();
                let len = len as usize;
                let mut data = vec![0u8; len];
                buf.copy_to_slice(&mut data[..]);
                Ok((EntryValue::RawData(data), len + size))
            }
            &EntryType::BooleanArray => {
                let mut bytes_read = 0;
                let len = buf.read_u8()? as usize;
                bytes_read += 1;
                let mut arr = vec![false; len];

                for i in 0..len {
                    let byte = buf.read_u8()?;
                    bytes_read += 1;
                    arr[i] = byte == 1;
                }

                Ok((EntryValue::BooleanArray(arr), bytes_read))
            }
            &EntryType::DoubleArray => {
                let mut bytes_read = 0;
                let len = buf.read_u8()? as usize;
                bytes_read += 1;
                let mut arr = vec![0f64; len];

                for i in 0..len {
                    let val = buf.read_f64_be()?;
                    bytes_read += 8;
                    arr[i] = val;
                }

                Ok((EntryValue::DoubleArray(arr), bytes_read))
            }
            &EntryType::StringArray => {
                let mut bytes_read = 0;

                let len = buf.read_u8()? as usize;
                bytes_read += 1;
                let mut arr = Vec::with_capacity(len);

                for i in 0..len {
                    let (val, bytes) = String::decode(buf)?;
                    bytes_read += bytes;
                    arr[i] = val;
                }

                Ok((EntryValue::StringArray(arr), bytes_read))
            }
            &EntryType::RPCDef => {
                let (rpc, bytes) = RPCDefinitionData::decode(buf)?;
                Ok((EntryValue::RPCDef(rpc), bytes))
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
                buf.write_unsigned(val.len() as u64).unwrap();
                buf.put_slice(&val[..]);
            }
            &EntryValue::BooleanArray(val) => {
                buf.write_unsigned(val.len() as u64).unwrap();

                for b in val {
                    buf.put_u8(*b as u8)
                }
            }
            &EntryValue::DoubleArray(val) => {
                buf.write_unsigned(val.len() as u64).unwrap();

                for d in val {
                    buf.put_f64_be(*d);
                }
            }
            &EntryValue::StringArray(val) => {
                buf.write_unsigned(val.len() as u64).unwrap();

                for s in val {
                    s.encode(buf);
                }
            }
            _ => panic!()
        }
    }
}

impl EntryValue {
    /// Returns the [`EntryType`] corresponding to the variant of [`self`]
    pub fn entry_type(&self) -> EntryType {
        match self {
            EntryValue::Boolean(_) => EntryType::Boolean,
            EntryValue::Double(_) => EntryType::Double,
            EntryValue::String(_) => EntryType::String,
            EntryValue::RawData(_) => EntryType::RawData,
            EntryValue::BooleanArray(_) => EntryType::BooleanArray,
            EntryValue::DoubleArray(_) => EntryType::DoubleArray,
            EntryValue::StringArray(_) => EntryType::StringArray,
            EntryValue::RPCDef(_) => EntryType::RPCDef,
            EntryValue::Pass => EntryType::RPCDef
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
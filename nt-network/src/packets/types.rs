use crate::ext::BufExt;
use crate::packets::Packet;
use crate::Result;
use bytes::{Buf, BufMut, BytesMut};
use anyhow::anyhow;
use thiserror::Error;
use nt_leb128::*;
#[cfg(feature = "wasm-bindgen")]
use wasm_bindgen::prelude::*;

impl Packet for String {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.write_unsigned(self.len() as u64).unwrap();
        buf.extend_from_slice(self.as_bytes());
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let (len, read) = {
            let (len, read) = buf.read_unsigned()?;
            (len as usize, read)
        };
        if buf.remaining() < len {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "String aint there",
            )
            .into());
        }
        let mut this = vec![0u8; len];
        buf.copy_to_slice(&mut this[..]);

        Ok((String::from_utf8(this).unwrap(), read + len))
    }
}

impl Packet for u8 {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(*self);
        Ok(())
    }

    fn deserialize(buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        Ok((buf.get_u8(), 1))
    }
}

impl Packet for bool {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(if *self { 1 } else { 0 });
        Ok(())
    }

    fn deserialize(buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        Ok((buf.get_u8() == 1, 1))
    }
}

impl Packet for f64 {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_f64(*self);
        Ok(())
    }

    fn deserialize(buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        Ok((buf.get_f64(), 8))
    }
}

impl<T: Packet> Packet for Vec<T> {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.write_unsigned(self.len() as u64)?;
        self.iter().for_each(|value| {
            value.serialize(buf).unwrap();
        });
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let (len, mut read) = buf.read_unsigned()?;
        let mut v = Vec::with_capacity(len as usize);

        for _ in 0..len {
            let (value, b) = T::deserialize(buf)?;
            v.push(value);
            read += b;
        }

        Ok((v, read))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RpcDefinition {
    V0,
}

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("Invalid RPC Definition: {version}")]
    InvalidVersion { version: u8 },
}

impl Packet for RpcDefinition {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        match *self {
            RpcDefinition::V0 => {
                buf.write_unsigned(1)?;
                buf.put_u8(0);
            }
        }
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let (len, read) = buf.read_unsigned()?;
        let ver = buf.get_u8();
        if len == 1 && ver == 0 {
            Ok((RpcDefinition::V0, len as usize + read))
        } else {
            Err(RpcError::InvalidVersion { version: ver }.into())
        }
    }
}

#[cfg_attr(feature = "wasm-bindgen", wasm_bindgen)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EntryType {
    Boolean,
    Double,
    String,
    RawData,
    BooleanArray,
    DoubleArray,
    StringArray,
    RpcDefinition,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EntryValue {
    Boolean(bool),
    Double(f64),
    String(String),
    RawData(Vec<u8>),
    BooleanArray(Vec<bool>),
    DoubleArray(Vec<f64>),
    StringArray(Vec<String>),
    RpcDefinition(RpcDefinition),
}

impl EntryValue {
    pub fn entry_type(&self) -> EntryType {
        match *self {
            EntryValue::Boolean(_) => EntryType::Boolean,
            EntryValue::Double(_) => EntryType::Double,
            EntryValue::String(_) => EntryType::String,
            EntryValue::RawData(_) => EntryType::RawData,
            EntryValue::BooleanArray(_) => EntryType::BooleanArray,
            EntryValue::DoubleArray(_) => EntryType::DoubleArray,
            EntryValue::StringArray(_) => EntryType::StringArray,
            EntryValue::RpcDefinition(_) => EntryType::RpcDefinition,
        }
    }
}

impl Packet for EntryType {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        match *self {
            EntryType::Boolean => buf.put_u8(0x00),
            EntryType::Double => buf.put_u8(0x01),
            EntryType::String => buf.put_u8(0x02),
            EntryType::RawData => buf.put_u8(0x03),
            EntryType::BooleanArray => buf.put_u8(0x10),
            EntryType::DoubleArray => buf.put_u8(0x11),
            EntryType::StringArray => buf.put_u8(0x12),
            EntryType::RpcDefinition => buf.put_u8(0x20),
        }
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let value = buf.read_u8()?;
        let entry = match value {
            0x00 => EntryType::Boolean,
            0x01 => EntryType::Double,
            0x02 => EntryType::String,
            0x03 => EntryType::RawData,
            0x10 => EntryType::BooleanArray,
            0x11 => EntryType::DoubleArray,
            0x12 => EntryType::StringArray,
            0x20 => EntryType::RpcDefinition,
            _ => return Err(anyhow!("Invalid entry type")),
        };

        Ok((entry, 1))
    }
}

impl EntryType {
    pub fn write_value(self, value: &EntryValue, buf: &mut BytesMut) -> Result<()> {
        match value {
            EntryValue::Boolean(ref b) => b.serialize(buf)?,
            EntryValue::Double(ref d) => d.serialize(buf)?,
            EntryValue::String(ref s) => s.serialize(buf)?,
            EntryValue::RawData(ref d) => d.serialize(buf)?,
            EntryValue::BooleanArray(ref v) => v.serialize(buf)?,
            EntryValue::DoubleArray(ref v) => v.serialize(buf)?,
            EntryValue::StringArray(ref v) => v.serialize(buf)?,
            EntryValue::RpcDefinition(ref v) => v.serialize(buf)?,
        }
        Ok(())
    }

    pub fn read_value(self, mut buf: &mut dyn Buf) -> Result<(EntryValue, usize)> {
        let mut read = 0;

        let value = match self {
            EntryType::Boolean => {
                read += 1;
                EntryValue::Boolean(buf.read_u8()? == 1)
            }
            EntryType::Double => {
                read += 8;
                EntryValue::Double(buf.read_f64_be()?)
            }
            EntryType::String => {
                let (s, len) = String::deserialize(buf)?;
                read += len;
                EntryValue::String(s)
            }
            EntryType::RawData => {
                let (v, len) = Vec::<u8>::deserialize(buf)?;
                read += len;
                EntryValue::RawData(v)
            }
            EntryType::BooleanArray => {
                let (v, len) = Vec::<bool>::deserialize(buf)?;
                read += len;
                EntryValue::BooleanArray(v)
            }
            EntryType::DoubleArray => {
                let (v, len) = Vec::<f64>::deserialize(buf)?;
                read += len;
                EntryValue::DoubleArray(v)
            }
            EntryType::StringArray => {
                let (v, len) = Vec::<String>::deserialize(buf)?;
                read += len;
                EntryValue::StringArray(v)
            }
            EntryType::RpcDefinition => {
                let (v, len) = Packet::deserialize(buf)?;
                read += len;
                EntryValue::RpcDefinition(v)
            }
        };
        Ok((value, read))
    }
}

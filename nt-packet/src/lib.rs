//! A crate containing base traits for interacting with NetworkTables packets in buffers
//!
//! This crate contains the traits core to `nt` and `nt-packet-derive` for reading and writing
//! values (Packets or other complex values) to and from byte buffers from the `bytes` crate.
//!
//! This is not meant to be used alone, and is merely here so that `nt` and `nt-packet-derive` are able to share the same
//! key traits.

#![deny(missing_docs)]

extern crate bytes;
extern crate nt_leb128 as leb128;
extern crate failure;

mod ext;

pub use self::ext::BufExt;

use bytes::{Buf, BytesMut, BufMut};

use leb128::write::LEB128Write;
use leb128::read::LEB128Read;

use std::io::{Error, ErrorKind};

/// Trait representing an NT message/packet headed Client --> Server
pub trait ClientMessage: Send {
    /// Encodes `Self` into the given `buf`
    fn encode(&self, buf: &mut BytesMut);
}

/// Trait representing an NT message/packet headed Server --> Client
pub trait ServerMessage: Sized {
    /// Attempts to decode `Self` from the given `buf`
    /// Returns Some if the given `buf` was a valid NT packet
    /// Returns None if the given `buf` was malformed, or otherwise invalid
    fn decode(buf: &mut Buf) -> Result<(Self, usize), failure::Error>;
}

impl ClientMessage for String {
    fn encode(&self, buf: &mut BytesMut) {
        buf.write_unsigned(self.len() as u64).unwrap();
        buf.put_slice(self.as_bytes());
    }
}

impl ServerMessage for String {
    fn decode(mut buf: &mut Buf) -> Result<(Self, usize), failure::Error> {
        let (len, bytes_read) = buf.read_unsigned()?;
        let len = len as usize;
        // bytes is silly and panics instead of returning an Err
        if buf.remaining() < len {
            return Err(Error::new(ErrorKind::UnexpectedEof, "string len > remaining").into());
        }
        let mut strbuf = vec![0; len];
        buf.copy_to_slice(&mut strbuf[..]);
        Ok((String::from_utf8(strbuf)?, len + bytes_read))
    }
}

impl ClientMessage for Vec<u8> {
    fn encode(&self, buf: &mut BytesMut) {
        buf.write_unsigned(self.len() as u64).unwrap();
        buf.extend_from_slice(&self[..]);
    }
}

impl<T> ClientMessage for Vec<T>
    where T: ClientMessage
{
    fn encode(&self, buf: &mut BytesMut) {
        let mut bytes = BytesMut::new();
        for item in self {
            item.encode(&mut bytes);
        }

        buf.write_unsigned(bytes.len() as u64).unwrap();
        buf.extend(bytes);
    }
}

impl<T> ServerMessage for Vec<T>
    where T: ServerMessage
{
    fn decode(mut buf: &mut Buf) -> Result<(Self, usize), failure::Error> {
        let mut bytes_read = 0;
        let len = {
            let (len, bytes) = buf.read_unsigned()?;

            bytes_read += bytes;

            len as usize
        };

        let mut vec = Vec::with_capacity(len as usize);
        for _ in 0..len {
            let (t, bytes) = T::decode(buf)?;
            bytes_read += bytes;
            vec.push(t)
        }

        Ok((vec, bytes_read))
    }
}

impl ServerMessage for Vec<u8> {
    fn decode(mut buf: &mut Buf) -> Result<(Self, usize), failure::Error> {
        let mut bytes_read = 0;
        let len = {
            let (len, bytes) = buf.read_unsigned()?;

            bytes_read += bytes;
            len as usize
        };
        let mut vec = vec![0; len];

        for i in 0..len {
            vec[i] = buf.read_u8()?;
            bytes_read += 1;
        }

        Ok((vec, bytes_read))
    }
}

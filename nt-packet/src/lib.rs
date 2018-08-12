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

use bytes::{Buf, BytesMut, BufMut};

use leb128::write::LEB128Write;
use leb128::read::LEB128Read;

/// Trait representing an NT message/packet headed Client --> Server
pub trait ClientMessage: Send {
    /// Encodes `Self` into the given `buf`
    fn encode(&self, buf: &mut BytesMut);
}

/// Trait representing an NT message/packet headed Server --> Client
pub trait ServerMessage {
    /// Attempts to decode `Self` from the given `buf`
    /// Returns Some if the given `buf` was a valid NT packet
    /// Returns None if the given `buf` was malformed, or otherwise invalid
    fn decode(buf: &mut Buf) -> (Option<Self>, usize)
        where Self: Sized;
}

impl ClientMessage for String {
    fn encode(&self, buf: &mut BytesMut) {
        buf.write_unsigned(self.len() as u64).unwrap();
        buf.put_slice(self.as_bytes());
    }
}

impl ServerMessage for String {
    fn decode(mut buf: &mut Buf) -> (Option<Self>, usize) {
        let (len, bytes_read) = buf.read_unsigned().unwrap();
        let len = len as usize;
        let mut strbuf = vec![0; len];
        buf.copy_to_slice(&mut strbuf[..]);
        (Some(::std::string::String::from_utf8(strbuf).unwrap()), len + bytes_read)
    }
}

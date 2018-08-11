extern crate bytes;
extern crate leb128;

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

extern crate bytes;

use bytes::{Buf, BytesMut, BufMut};

mod leb128;

/// Trait representing an NT message/packet headed Client --> Server
pub trait ClientMessage {
    /// Encodes `Self` into the given `buf`
    fn encode(&self, buf: &mut BytesMut);
}

/// Trait representing an NT message/packet headed Server --> Client
pub trait ServerMessage {
    /// Attempts to decode `Self` from the given `buf`
    /// Returns Some if the given `buf` was a valid NT packet
    /// Returns None if the given `buf` was malformed, or otherwise invalid
    fn decode(buf: &mut Buf) -> Option<Self>
        where Self: Sized;
}

impl ClientMessage for String {
    fn encode(&self, buf: &mut BytesMut) {
        ::leb128::write(buf, self.len() as u64);
        buf.put_slice(self.as_bytes());
    }
}

impl ServerMessage for String {
    fn decode(buf: &mut Buf) -> Option<Self> {
        let len = ::leb128::read(buf) as usize;
        let mut strbuf = vec![0; len];
        buf.copy_to_slice(&mut strbuf[..]);
        Some(::std::string::String::from_utf8(strbuf).unwrap())
    }
}

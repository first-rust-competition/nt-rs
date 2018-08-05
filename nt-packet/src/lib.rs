extern crate bytes;

use bytes::{Buf, BytesMut};

/// Enum representing the type of an NT message
/// Can be encoded using the given `Into<u8>` impl
pub enum MessageType {
    ClientHello
}

impl Into<u8> for MessageType {
    fn into(self) -> u8 {
        match self {
            MessageType::ClientHello => 0x01
        }
    }
}

/// Trait representing an NT message/packet headed Client --> Server
pub trait ClientMessage {
    /// Encodes `Self` into the given `buf`
    fn encode(&self, buf: &mut BytesMut);
}

/// Trait representing an NT message/packet headed Server --> Client
pub trait ServerMessage: Sized {
     /// Attempts to decode `Self` from the given `buf`
    /// Returns Some if the given `buf` was a valid NT packet
    /// Returns None if the given `buf` was malformed, or otherwise invalid
    fn decode(buf: &mut Buf) -> Option<Self>;
}
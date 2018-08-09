use tokio_codec::{Encoder, Decoder};
use bytes::{BytesMut, IntoBuf};

use nt_packet::ClientMessage;
use super::Packet;

use std::io::Error;

use super::try_decode;

/// Codec for the NetworkTables protocol
/// Built on `ClientMessage` for outgoing packets, and `ServerMessage` for incoming packets.
pub struct NTCodec;

impl Encoder for NTCodec {
    type Item = Box<ClientMessage>;
    type Error = Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.encode(dst);
        Ok(())
    }
}

impl Decoder for NTCodec {
    type Item = Packet;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut buf = src.clone().freeze().into_buf();
        let (packet, bytes_read) = try_decode(&mut buf);

        // This makes sure that a value was actually read successfully from the buffer, so that advancing the cursor is fine
        if packet.is_some() {
            src.advance(bytes_read);
        }

        Ok(packet)
    }
}
use tokio_codec::{Encoder, Decoder};
use bytes::{BytesMut, IntoBuf, Buf};

use nt::state::State;

use std::sync::{Arc, Mutex};

use nt_packet::{ClientMessage, ServerMessage};
use super::Packet;

use std::io::Error;

use super::try_decode;

pub trait ServerMessageStateful: ServerMessage {
    fn decode_stateful(buf: &mut Buf, state: Arc<Mutex<State>>) -> (Option<Self>, usize)
        where Self: Sized;
}

/// Codec for the NetworkTables protocol
/// Built on `ClientMessage` for outgoing packets, and `ServerMessage` for incoming packets.
pub struct NTCodec {
    state: Arc<Mutex<State>>
}

impl NTCodec {
    pub fn new(state: Arc<Mutex<State>>) -> NTCodec {
        NTCodec {
            state
        }
    }
}

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
        let (packet, bytes_read) = try_decode(&mut buf, &self.state);

        // This makes sure that a value was actually read successfully from the buffer, so that advancing the cursor is fine
        if packet.is_some() {
            src.advance(bytes_read);
        }

        Ok(packet)
    }
}
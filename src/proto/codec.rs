use tokio_codec::{Encoder, Decoder};
use bytes::{BytesMut, IntoBuf, Buf};

use nt::state::State;

use std::sync::{Arc, Mutex};

use nt_packet::ClientMessage;
use super::Packet;

use std::io;

use super::try_decode;

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
    type Error = ::failure::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.encode(dst);
        Ok(())
    }
}

impl Decoder for NTCodec {
    type Item = Packet;
    type Error = ::failure::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut buf = src.clone().freeze().into_buf();

        if buf.remaining() < 1 {
            return Ok(None);
        }

        let (packet, bytes_read) = match try_decode(&mut buf, &self.state) {
            Ok(t) => t,
            Err(e) => match e.find_root_cause().downcast_ref::<io::Error>() {
                Some(err) => if err.kind() == io::ErrorKind::UnexpectedEof {
                    return Ok(None);
                }else {
                    return Err(e);
                },
                None => return Err(e)
            }
        };

        // Advance the buffer, any errors in deserializing that would cause us to not want to advance it are handled in the match above
        src.advance(bytes_read);

        Ok(Some(packet))
    }
}
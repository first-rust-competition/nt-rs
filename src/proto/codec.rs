use tokio_codec::{Encoder, Decoder};
use bytes::{BytesMut, IntoBuf};
use failure;

use nt_packet::{ClientMessage, ServerMessage};

use super::try_decode;

pub struct NTCodec;

impl Encoder for NTCodec {
    type Item = Box<ClientMessage>;
    type Error = failure::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.encode(dst);
        Ok(())
    }
}

impl Decoder for NTCodec {
    type Item = Box<ServerMessage>;
    type Error = failure::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(try_decode(&mut src.freeze().clone().into_buf()))
    }
}
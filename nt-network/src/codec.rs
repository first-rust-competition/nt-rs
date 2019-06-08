use tokio_codec::{Encoder, Decoder};
use bytes::{BytesMut, Buf, IntoBuf};
use crate::ext::*;
use crate::{Result, ClientHello, ProtocolVersionUnsupported, ServerHello, EntryAssignment, EntryUpdate, EntryFlagsUpdate, EntryDelete, ClearAllEntries, Packet};
use std::io;

#[derive(Clone, Debug)]
pub enum ReceivedPacket {
    KeepAlive,
    ClientHello(ClientHello),
    ProtocolVersionUnsupported(ProtocolVersionUnsupported),
    ServerHelloComplete,
    ServerHello(ServerHello),
    ClientHelloComplete,
    EntryAssignment(EntryAssignment),
    EntryUpdate(EntryUpdate),
    EntryFlagsUpdate(EntryFlagsUpdate),
    EntryDelete(EntryDelete),
    ClearAllEntries(ClearAllEntries)
}

pub struct NTCodec;

impl Encoder for NTCodec {
    type Item = Box<dyn Packet>;
    type Error = failure::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<()> {

        dst.put_serializable(&*item);
        Ok(())
    }
}

impl Decoder for NTCodec {
    type Item = ReceivedPacket;
    type Error = failure::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<ReceivedPacket>> {
        let mut buf = src.clone().freeze().into_buf();

        if buf.remaining() < 1 {
            return Ok(None);
        }

        let (packet, bytes) = match try_decode(&mut buf) {
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

        src.advance(bytes);
        Ok(Some(packet))
    }
}

fn try_decode(mut buf: &mut dyn Buf) -> Result<(ReceivedPacket, usize)> {
    let id = buf.read_u8()?;

    let mut bytes = 1;

    let packet = match id {
        0x00 => Some(ReceivedPacket::KeepAlive),
        0x01 => {
            let (packet, read) = ClientHello::deserialize(buf)?;
            bytes += read;
            Some(ReceivedPacket::ClientHello(packet))
        }
        0x02 => {
            let (packet, read) = ProtocolVersionUnsupported::deserialize(buf)?;
            bytes += read;
            Some(ReceivedPacket::ProtocolVersionUnsupported(packet))
        }
        0x03 => Some(ReceivedPacket::ServerHelloComplete),
        0x04 => {
            let (packet, read) = ServerHello::deserialize(buf)?;
            bytes += read;
            Some(ReceivedPacket::ServerHello(packet))
        }
        0x05 => Some(ReceivedPacket::ClientHelloComplete),
        0x10 => {
            let (packet, read) = EntryAssignment::deserialize(buf)?;
            bytes += read;
            Some(ReceivedPacket::EntryAssignment(packet))
        }
        0x11 => {
            let (packet, read) = EntryUpdate::deserialize(buf)?;
            bytes += read;
            Some(ReceivedPacket::EntryUpdate(packet))
        }
        0x12 => {
            let (packet, read) = EntryFlagsUpdate::deserialize(buf)?;
            bytes += read;
            Some(ReceivedPacket::EntryFlagsUpdate(packet))
        }
        0x13 => {
            let (packet, read) = EntryDelete::deserialize(buf)?;
            bytes += read;
            Some(ReceivedPacket::EntryDelete(packet))
        }
        0x14 => {
            let (packet, read) = ClearAllEntries::deserialize(buf)?;
            bytes += read;
            Some(ReceivedPacket::ClearAllEntries(packet))
        }
        _ => None
    };

    Ok((packet.unwrap(), bytes))
}


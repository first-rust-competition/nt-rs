use crate::ext::*;
use crate::{
    ClearAllEntries, ClientHello, EntryAssignment, EntryDelete, EntryFlagsUpdate, EntryUpdate,
    Packet, ProtocolVersionUnsupported, Result, ServerHello,
};
use bytes::{Buf, BytesMut};
use std::io;
use tokio_util::codec::{Decoder, Encoder};

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
    ClearAllEntries(ClearAllEntries),
    RpcExecute(RpcExecute),
    RpcResponse(RpcResponse),
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
        let mut buf = src.clone().freeze();

        if buf.remaining() < 1 {
            return Ok(None);
        }

        let (packet, bytes) = match try_decode(&mut buf) {
            Ok(t) => t,
            Err(e) => match e.find_root_cause().downcast_ref::<io::Error>() {
                Some(err) => {
                    if err.kind() == io::ErrorKind::UnexpectedEof {
                        return Ok(None);
                    } else {
                        return Err(e);
                    }
                }
                None => return Err(e),
            },
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
        },
        0x20 => {
            let (packet, read) = RpcExecute::deserialize(buf)?;
            bytes += read;
            Some(ReceivedPacket::RpcExecute(packet))
        },
        0x21 => {
            let (packet, read) = RpcResponse::deserialize(buf)?;
            bytes += read;
            Some(ReceivedPacket::RpcResponse(packet))
        }
        _ => None,
    };

    match packet {
        Some(packet) => Ok((packet, bytes)),
        None => failure::bail!("Failed to decode packet"),
    }
}

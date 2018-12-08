pub mod server;
pub mod client;
pub mod types;
pub mod codec;
pub mod rpc;

use self::types::*;
use self::rpc::RPCResponse;
use crate::nt::state::State;

use std::sync::{Arc, Mutex};

use bytes::Buf;
use nt_packet::*;
use self::server::*;

/// Represents an attempt to decode a `ServerMessage` from the given `buf`
pub fn try_decode(mut buf: &mut Buf, state: &Arc<Mutex<State>>) -> Result<(Packet, usize), ::failure::Error> {
    use self::server::*;

    let id = buf.read_u8()?;

    // The total bytes read, will differ with the different types of packets that are read.
    // Updated appropriately in the `match` so that the caller can update the byte source accordingly
    let mut bytes = 1;

    let packet = match id {
        0x00 => {
            let (packet, bytes_read) = KeepAlive::decode(buf)?;
            bytes += bytes_read;
            Some(Packet::KeepAlive(packet))
        }
        0x02 => {
            let (packet, bytes_read) = ProtocolVersionUnsupported::decode(buf)?;
            bytes += bytes_read;
            Some(Packet::ProtocolVersionUnsupported(packet))
        }
        0x04 => {
            let (packet, bytes_read) = ServerHello::decode(buf)?;
            bytes += bytes_read;
            Some(Packet::ServerHello(packet))
        }
        0x03 => {
            let (packet, bytes_read) = ServerHelloComplete::decode(buf)?;
            bytes += bytes_read;
            Some(Packet::ServerHelloComplete(packet))
        }
        0x10 => {
            let (packet, bytes_read) = EntryAssignment::decode(buf)?;
            bytes += bytes_read;
            Some(Packet::EntryAssignment(packet))
        }
        0x13 => {
            let (packet, bytes_read) = EntryDelete::decode(buf)?;
            bytes += bytes_read;
            Some(Packet::EntryDelete(packet))
        }
        0x11 => {
            let (packet, bytes_read) = EntryUpdate::decode(buf)?;
            bytes += bytes_read;
            Some(Packet::EntryUpdate(packet))
        }
        0x12 => {
            let (packet, bytes_read) = EntryFlagsUpdate::decode(buf)?;
            bytes += bytes_read;
            Some(Packet::EntryFlagsUpdate(packet))
        }
        0x14 => {
            let (packet, bytes_read) = DeleteAllEntries::decode(buf)?;
            bytes += bytes_read;
            Some(Packet::DeleteAllEntries(packet))
        }
        0x21 => {
            let (packet, bytes_read) = RPCResponse::decode(buf, state)?;
            bytes += bytes_read;
            Some(Packet::RpcResponse(packet))
        }
        _ => None
    };
    let packet = match packet {
        Some(packet) => packet,
        None => bail!("Invalid packet received"),
    };

    Ok((packet, bytes))
}

/// Enum wrapping `ServerMessage` types
#[derive(Debug)]
pub enum Packet {
    KeepAlive(KeepAlive),
    ProtocolVersionUnsupported(ProtocolVersionUnsupported),
    ServerHello(ServerHello),
    ServerHelloComplete(ServerHelloComplete),
    EntryAssignment(EntryAssignment),
    EntryDelete(EntryDelete),
    EntryUpdate(EntryUpdate),
    EntryFlagsUpdate(EntryFlagsUpdate),
    DeleteAllEntries(DeleteAllEntries),
    RpcResponse(RPCResponse),
}

/// Represents NT Packet 0x00 Keep Alive
/// Sent by the client to ensure that a connection is still valid
#[derive(Debug, ClientMessage, ServerMessage)]
#[packet_id = 0x00]
pub struct KeepAlive;

/// Represents NT packet 0x13 Entry Delete
/// Sent by a server when a peer has deleted a value with id `entry_id`
/// Sent by a client to update the server to delete a value with id `entry_id`
#[derive(Debug, ClientMessage, ServerMessage, new)]
#[packet_id = 0x13]
pub struct EntryDelete {
    pub entry_id: u16,
}

#[derive(Debug, ClientMessage, ServerMessage)]
#[packet_id = 0x14]
pub struct DeleteAllEntries {
    pub magic: u32,
}

impl DeleteAllEntries {
    pub fn new() -> DeleteAllEntries {
        DeleteAllEntries {
            magic: 0xD06CB27A
        }
    }
}

#[derive(Debug,Clone, ClientMessage, ServerMessage, new)]
#[packet_id = 0x12]
pub struct EntryFlagsUpdate {
    pub entry_id: u16,
    pub entry_flags: u8,
}

#[derive(Debug, ClientMessage, new, Clone)]
#[packet_id = 0x11]
pub struct EntryUpdate {
    pub entry_id: u16,
    pub entry_sequence_num: u16,
    pub entry_type: EntryType,
    pub entry_value: EntryValue,
}

impl ServerMessage for EntryUpdate {
    fn decode(mut buf: &mut Buf) -> Result<(Self, usize), ::failure::Error> {
        let mut bytes_read = 0;
        let entry_id = buf.read_u16_be()?;
        bytes_read += 2;
        let entry_sequence_num = buf.read_u16_be()?;
        bytes_read += 2;
        let (entry_type, bytes) = EntryType::decode(buf)?;
        bytes_read += bytes;
        let (entry_value, bytes) = entry_type.get_entry(buf)?;
        bytes_read += bytes;

        Ok((EntryUpdate {
            entry_id,
            entry_sequence_num,
            entry_type,
            entry_value,
        }, bytes_read))
    }
}

/// Represents NT packet 0x10 Entry Assignment
/// Due to the non-deterministic nature of decoding `EntryValue`, manual implementation was required
#[derive(Debug, ClientMessage)]
#[packet_id = 0x10]
pub struct EntryAssignment {
    pub entry_name: String,
    pub entry_type: EntryType,
    pub entry_id: u16,
    pub entry_sequence_num: u16,
    pub entry_flags: u8,
    pub entry_value: EntryValue,
}

impl EntryAssignment {
    pub fn new(name: &str, ty: EntryType, id: u16, sequence_num: u16, flags: u8, value: EntryValue) -> EntryAssignment {
        EntryAssignment {
            entry_name: name.to_string(),
            entry_type: ty,
            entry_id: id,
            entry_sequence_num: sequence_num,
            entry_flags: flags,
            entry_value: value,
        }
    }
}

impl ServerMessage for EntryAssignment {
    fn decode(mut buf: &mut Buf) -> Result<(Self, usize), ::failure::Error> {
        // Tally of the total bytes read from `buf` to decode this packet
        let mut bytes_read = 0;

        let entry_name = {
            let (s, bytes) = String::decode(buf)?;
            bytes_read += bytes;
            s
        };

        let entry_type = {
            let (et, bytes) = EntryType::decode(buf)?;
            bytes_read += bytes;
            et
        };

        let entry_id = buf.read_u16_be()?;
        bytes_read += 2;

        let entry_sequence_num = buf.read_u16_be()?;
        bytes_read += 2;

        let entry_flags = buf.read_u8()?;
        bytes_read += 1;

        let (entry_value, bytes) = entry_type.get_entry(buf)?;
        bytes_read += bytes;

        Ok((EntryAssignment {
            entry_name,
            entry_type,
            entry_id,
            entry_sequence_num,
            entry_flags,
            entry_value,
        }, bytes_read))
    }
}
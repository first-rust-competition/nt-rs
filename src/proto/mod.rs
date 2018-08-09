pub mod server;
pub mod client;
pub mod types;
pub mod codec;

use self::types::*;

use bytes::{Buf, BufMut, BytesMut};
use nt_packet::*;
use self::server::*;

/// Represents an attempt to decode a `ServerMessage` from the given `buf`
pub fn try_decode(buf: &mut Buf) -> (Option<Packet>, usize) {
    use self::server::*;
    // Safety net to not read if there's nothing there
    if buf.remaining() < 1 {
        return (None, 0);
    }

    let id = buf.get_u8();

    // The total bytes read, will differ with the different types of packets that are read.
    // Updated appropriately in the `match` so that the caller can update the byte source accordingly
    let mut bytes = 1;

    let packet = match id {
        0x00 =>  {
            let (packet, bytes_read) = KeepAlive::decode(buf);
            bytes += bytes_read;
            Some(Packet::KeepAlive(packet.unwrap()))
        },
        0x02 => {
            let (packet, bytes_read) = ProtocolVersionUnsupported::decode(buf);
            bytes += bytes_read;
            Some(Packet::ProtocolVersionUnsupported(packet.unwrap()))
        },
        0x04 => {
            let (packet, bytes_read) = ServerHello::decode(buf);
            bytes += bytes_read;
            Some(Packet::ServerHello(packet.unwrap()))
        },
        0x03 => {
            let (packet, bytes_read) = ServerHelloComplete::decode(buf);
            bytes += bytes_read;
            Some(Packet::ServerHelloComplete(packet.unwrap()))
        },
        0x10 => {
            let (packet, bytes_read) = EntryAssignment::decode(buf);
            bytes += bytes_read;
            Some(Packet::EntryAssignment(packet.unwrap()))
        }
        0x13 => {
            let (packet, bytes_read) = EntryDelete::decode(buf);
            bytes += bytes_read;
            Some(Packet::EntryDelete(packet.unwrap()))
        }
        _ => None
    };

    (packet, bytes)
}

/// Enum wrapping `ServerMessage` types
pub enum Packet {
    KeepAlive(KeepAlive),
    ProtocolVersionUnsupported(ProtocolVersionUnsupported),
    ServerHello(ServerHello),
    ServerHelloComplete(ServerHelloComplete),
    EntryAssignment(EntryAssignment),
    EntryDelete(EntryDelete),
}

/// Represents NT Packet 0x00 Keep Alive
/// Sent by the client to ensure that a connection is still valid
#[derive(Debug, ClientMessage, ServerMessage)]
#[packet_id = 0x00]
pub struct KeepAlive;

/// Represents NT packet 0x13 Entry Delete
/// Sent by a server when a peer has deleted a value with id `entry_id`
/// Sent by a client to update the server to delete a value with id `entry_id`
#[derive(Debug, ClientMessage, ServerMessage)]
#[packet_id = 0x13]
pub struct EntryDelete {
    pub entry_id: u16,
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
            entry_value: value
        }
    }
}

impl ServerMessage for EntryAssignment {
    fn decode(buf: &mut Buf) -> (Option<Self>, usize) {
        // Tally of the total bytes read from `buf` to decode this packet
        let mut bytes_read = 0;

        let entry_name = {
            let (s, bytes) = String::decode(buf);
            bytes_read += bytes;
            s.unwrap()
        };

        let entry_type = {
            let (et, bytes) = EntryType::decode(buf);
            bytes_read += bytes;
            et.unwrap()
        };

        let entry_id = buf.get_u16_be();
        bytes_read += 2;

        let entry_sequence_num = buf.get_u16_be();
        bytes_read += 2;

        let entry_flags = buf.get_u8();
        bytes_read += 1;

        let (entry_value, bytes) = entry_type.get_entry(buf);
        bytes_read += bytes;

        (Some(EntryAssignment {
            entry_name,
            entry_type,
            entry_id,
            entry_sequence_num,
            entry_flags,
            entry_value,
        }), bytes_read)
    }
}
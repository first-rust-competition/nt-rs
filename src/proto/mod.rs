pub mod server;
pub mod client;
pub mod types;
pub mod codec;

use self::types::*;

use bytes::Buf;
use nt_packet::ServerMessage;
use self::server::*;

pub fn try_decode(buf: &mut Buf) -> (Option<Packet>, usize) {
    use self::server::*;
    // Safety net to not read if there's nothing there
    if buf.remaining() < 1 {
        return (None, 0);
    }

    let id = buf.get_u8();

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
        _ => None
    };

    (packet, bytes)
}

pub enum Packet {
    KeepAlive(KeepAlive),
    ProtocolVersionUnsupported(ProtocolVersionUnsupported),
    ServerHello(ServerHello),
    ServerHelloComplete(ServerHelloComplete),
    EntryAssignment(EntryAssignment),
}

#[derive(Debug, ClientMessage, ServerMessage)]
#[packet_id = 0x00]
pub struct KeepAlive;

/// Packet ID 0x10 (Cannot derive due to non-deterministic nature of last field)
#[derive(Debug)]
pub struct EntryAssignment {
    entry_name: String,
    entry_type: EntryType,
    entry_id: [u8; 2],
    entry_sequence_num: [u8; 2],
    entry_flags: u8,
    entry_value: EntryValue,
}

impl ServerMessage for EntryAssignment {
    fn decode(buf: &mut Buf) -> (Option<Self>, usize) {
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
        let entry_id = {
            let mut entry_buf = [0u8; 2];
            buf.copy_to_slice(&mut entry_buf[..]);
            entry_buf
        };
        bytes_read += 2;

        let entry_sequence_num = {
            let mut entry_buf = [0u8; 2];
            buf.copy_to_slice(&mut entry_buf[..]);
            entry_buf
        };
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
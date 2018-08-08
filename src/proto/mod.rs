pub mod server;
pub mod client;
pub mod types;
pub mod codec;

use self::types::*;

use bytes::Buf;
use nt_packet::ServerMessage;

pub fn try_decode<T>(buf: &mut Buf) -> Option<Box<ServerMessage>> {
    use self::server::*;
    let id = buf.get_u8();

    let packet= match id {
        0x00 => Box::new(KeepAlive::decode(buf)),
        0x02 => Box::new(ProtocolVersionUnsupported::decode(buf)),
        0x04 => Box::new(ServerHello::decode(buf)),
        0x03 => Box::new(ServerHelloComplete::decode(buf)),
        _ => Box::new(None)
    };

    packet.map(|it| Box::new(it))
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


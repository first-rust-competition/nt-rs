#![feature(attr_literals)]

extern crate futures;
extern crate tokio;
extern crate bytes;
extern crate nt_packet;
#[macro_use]
extern crate nt_packet_derive;

pub const NT_PROTOCOL_REV: u16 = 0x0300;

mod hello;

use bytes::{BytesMut, IntoBuf};

use nt_packet::*;
use hello::*;

mod leb128;

fn main() {
    let packet = ClientHello::new(NT_PROTOCOL_REV, "nt-rs-client");
    let mut buf = BytesMut::new();
    packet.encode(&mut buf);
}

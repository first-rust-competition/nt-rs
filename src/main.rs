#![feature(attr_literals)]

extern crate futures;
extern crate tokio;
extern crate tokio_codec;
extern crate bytes;
extern crate nt_packet;
#[macro_use]
extern crate nt_packet_derive;
extern crate failure;

pub const NT_PROTOCOL_REV: u16 = 0x0300;

pub type Result<T> = std::result::Result<T, failure::Error>;

mod proto;

use proto::codec::NTCodec;

use bytes::{BytesMut, IntoBuf, BufMut};

use nt_packet::*;
use tokio::net::TcpStream;
use tokio::io::AsyncRead;
use futures::Future;

mod leb128;

fn main() -> Result<()>{
    let client = TcpStream::connect(&"127.0.0.1:1735".parse()?)
        .map(|sock| sock.framed(NTCodec));

    Ok(())
}

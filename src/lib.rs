#![feature(attr_literals, nll)]

extern crate futures;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_core;
extern crate bytes;
extern crate nt_packet;
#[macro_use]
extern crate nt_packet_derive;
extern crate failure;
#[macro_use]
extern crate log;

pub const NT_PROTOCOL_REV: u16 = 0x0300;

pub type Result<T> = std::result::Result<T, failure::Error>;

mod proto;
mod nt;

pub use self::proto::types::{EntryData, EntryValue, EntryType};
pub use self::nt::*;

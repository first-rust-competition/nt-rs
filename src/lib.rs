//! # nt
//!
//! `nt` is a client library to the NetworkTables revision 3 protocol, backed by tokio.
//!
//! The provided [`NetworkTables`](struct.NetworkTables.html) struct contains methods for querying
//! the state of the connection, accessing, as well as updating and creating entries that will be
//! synced to the server.

#![feature(nll)]
#![deny(missing_docs)]

extern crate futures;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_core;
extern crate bytes;
extern crate nt_packet;
#[macro_use]
extern crate nt_packet_derive;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate nt_leb128 as leb128;
#[macro_use]
extern crate derive_new;
extern crate multimap;

#[doc(hidden)]
pub const NT_PROTOCOL_REV: u16 = 0x0300;

mod proto;
mod nt;

/// Base result type for nt-rs
pub type Result<T> = std::result::Result<T, failure::Error>;

pub use self::proto::types::{EntryData, EntryValue, EntryType, rpc::*};
pub use self::nt::*;

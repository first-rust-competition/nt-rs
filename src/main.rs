#![feature(attr_literals, nll)]

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
mod nt;

use proto::codec::NTCodec;
use proto::client::*;
use proto::Packet;

use tokio::net::TcpStream;
use tokio_codec::Decoder;
use futures::{Stream, Sink, Future};
use futures::future::ok;
use futures::future::Either;

use std::sync::{Arc, Mutex};

use nt::*;
use nt::state::*;

fn main() -> Result<()> {
    // Core state of the application. Contains state of the connection, and entries gotten from the server
    let mut state = Arc::new(Mutex::new(State::new()));

    // Open the initial connection. Once our first packet has been sent, spawn a new process to poll the receiver for any new data
    let client = NetworkTables::connect("nt-rs", &"127.0.0.1:1735".parse()?, state.clone())
        .and_then(move |codec| tokio::spawn(poll(state.clone(), codec)));

    // Start the client, will block until the connection is closed.
    tokio::run(client);

    Ok(())
}

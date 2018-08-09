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

use futures::{Future, Stream};
use futures::sync::mpsc::channel;

use std::sync::{Arc, Mutex};

use nt::*;
use nt::state::*;

fn main() -> Result<()> {
    // Core state of the application. Contains state of the connection, and entries gotten from the server
    let state = Arc::new(Mutex::new(State::new()));

    // Open the initial connection. Once our first packet has been sent, spawn a new process to poll the receiver for any new data
//    let client = NetworkTables::connect("nt-rs", &"127.0.0.1:1735".parse()?, state.clone())
//        .and_then(move |codec| tokio::spawn(poll(state.clone(), codec)));

    let client = NetworkTables::connect("nt-rs", &"127.0.0.1:1735".parse()?, state.clone())
        .and_then(move |codec| {
            let (tx, rx) = codec.split();
            let (chan_tx, chan_rx) = channel(5);

            tokio::spawn(send_packets(tx, chan_rx));
            tokio::spawn(poll_socket(state.clone(), rx, chan_tx.clone()));
            tokio::spawn(poll_stdin(state.clone(), chan_tx.clone()))
        });

    // Start the client, will block until the connection is closed.
    tokio::run(client);

    Ok(())
}

use std::net::SocketAddr;
use futures::{Future, Poll};
use futures::future::{ok, Either};
use tokio::prelude::*;
use tokio::net::TcpStream;
use tokio_codec::{Decoder, Framed};
use tokio::timer::Interval;
use tokio;

use std::time::{Instant, Duration};

use proto::codec::NTCodec;
use proto::client::*;
use proto::*;
use proto::server::*;
use proto::types::*;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub mod state;

use state::*;

/// Core struct representing a connection to a NetworkTables server
pub struct NetworkTables {
    /// Represents the current state of the connection
    state: Arc<Mutex<State>>,
    /// Contains the initial connection future with a reference to the framed NT codec
    handle: Box<Future<Item=Framed<TcpStream, NTCodec>, Error=()> + Send>,
}

/// Implementation of Future delegating to `self.handle`
impl Future for NetworkTables {
    type Item = Framed<TcpStream, NTCodec>;
    type Error = ();

    fn poll(&mut self) -> Poll<Framed<TcpStream, NTCodec>, ()> {
        self.handle.poll()
    }
}

impl NetworkTables {
    /// Performs the initial connection process to the given `target`.
    /// Assumes that target is a valid, running NetworkTables server.
    /// Returns a `NetworkTables` ready to be started on the tokio runtime to initialize the connection
    pub fn connect(client_name: &'static str, target: &SocketAddr, state: Arc<Mutex<State>>) -> NetworkTables {
        state.lock().unwrap().set_state(ConnectionState::Connecting);

        // Internal handle of the struct
        let future = TcpStream::connect(target)
            .and_then(move |sock| {
                // Once we have the socket, frame it with the NT codec and send the first packet to begin the connection
                let codec = NTCodec.framed(sock);
                codec.send(Box::new(ClientHello::new(::NT_PROTOCOL_REV, client_name)))
            })
            .map_err(|_| ());

        NetworkTables {
            state,
            handle: Box::new(future),
        }
    }
}

/// Function containing the future for polling the remote peer for new packets
/// Expects to be started with `tokio::spawn`
/// Mutates the `state` as packets are received
pub fn poll(state: Arc<Mutex<State>>, codec: Framed<TcpStream, NTCodec>) -> impl Future<Item=(), Error=()> {
    let (tx, rx) = codec.split();
    // This function will be called as new packets arrive. Has to be `fold` to maintain ownership over the write half of our codec
    rx.fold(tx, move |tx, packet| {
        // Perform certain actions based on the internal packet that was decoded.
        // A value of Either::B() represents something where no packet was sent in response
        // A value of Either::A() represents something where a packet had to be sent
        match packet {
            Packet::ServerHello(packet) => {
                println!("Got server hello: {:?}", packet);
                Either::B(ok(tx))
            }
            Packet::ServerHelloComplete(_) => {
                println!("Got server hello complete");
                state.lock().unwrap().set_state(ConnectionState::Connected);
                println!("Sent ClientHelloComplete");
                Either::A(tx.send(Box::new(ClientHelloComplete)))
            }
            Packet::ProtocolVersionUnsupported(packet) => {
                println!("Got this {:?}", packet);
                Either::B(ok(tx))
            }
            Packet::EntryAssignment(entry /* heheheh */) => {
                let mut state = state.lock().unwrap();
                state.add_entry(entry.entry_id, entry.entry_value);

                Either::B(ok(tx))
            }
            Packet::EntryDelete(delet) => {
                let mut state = state.lock().unwrap();
                state.remove_entry(delet.entry_id);

                Either::B(ok(tx))
            }
            _ => Either::B(ok(tx))
        }
    })
        .map_err(|e| println!("Got error {:?}", e))
        .map(|_| ())
}

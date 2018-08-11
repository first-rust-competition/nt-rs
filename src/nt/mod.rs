use std::net::SocketAddr;
use futures::{Future, Poll};
use futures::future::{ok, Either};
use futures::sync::mpsc::{Sender, Receiver, channel};
use tokio::prelude::*;
use tokio::net::TcpStream;
use tokio_codec::{Decoder, Framed, FramedRead, LinesCodec};
use tokio::io::stdin;
use tokio;
use tokio_core::reactor::Core;

use nt_packet::ClientMessage;
use proto::codec::NTCodec;
use proto::client::*;
use proto::*;
use proto::types::*;

use std::sync::{Arc, Mutex};
use std::io::Error;
use std::collections::HashMap;
use std::thread;

pub mod state;
mod conn;
mod handler;

use self::handler::*;
use self::state::*;
use self::conn::Connection;

/// Core struct representing a connection to a NetworkTables server
pub struct NetworkTables {
    /// Contains the initial connection future with a reference to the framed NT codec
    state: Arc<Mutex<State>>,
}

impl NetworkTables {
    /// Performs the initial connection process to the given `target`.
    /// Assumes that target is a valid, running NetworkTables server.
    /// Returns a `NetworkTables`. The state of the connection can be told by the contained `State`
    pub fn connect(client_name: &'static str, target: SocketAddr) -> NetworkTables {
        let state = Arc::new(Mutex::new(State::new()));
        state.lock().unwrap().set_connection_state(ConnectionState::Connecting);

        let thread_state = state.clone();
        let _ = thread::spawn(move || {
            let mut core = Core::new().unwrap();
            let handle = core.handle();
            let state = thread_state;
            {
                state.clone().lock().unwrap().set_handle(handle.clone().remote().clone());
            }

            core.run(Connection::new(&handle, &target, client_name, state)).unwrap()
        });

        NetworkTables {
            state,
        }
    }

    pub fn state(&self) -> &Arc<Mutex<State>> {
        &self.state
    }

    pub fn create_entry(&mut self, data: EntryData) {
        self.state.lock().unwrap().create_entry(data);
    }

    pub fn connected(&self) -> bool {
        self.state.lock().unwrap().connection_state().connected()
    }
}

pub fn send_packets(tx: impl Sink<SinkItem=Box<ClientMessage>, SinkError=Error>, rx: Receiver<Box<ClientMessage>>) -> impl Future<Item=(), Error=()> {
    info!("Spawned packet send loop");
    rx
        .map_err(|_| ())
        .fold(tx, |tx, packet| tx.send(packet).map_err(|_| ()))
        .then(|_| Ok(()))
}

/// Function containing the future for polling the remote peer for new packets
/// Expects to be started with `tokio::spawn`
/// Mutates the `state` as packets are received
pub fn poll_socket(state: Arc<Mutex<State>>, rx: impl Stream<Item=Packet, Error=Error>, tx: Sender<Box<ClientMessage>>) -> impl Future<Item=(), Error=()> {
    info!("Spawned socket poll");

    // This function will be called as new packets arrive. Has to be `fold` to maintain ownership over the write half of our codec
    rx
        .map_err(drop)
        .fold(tx, move |tx, packet| {
            match handle_packet(packet, state.clone(), tx.clone()) {
                Some(packet) => Either::A(tx.send(packet).map_err(drop)),
                None => Either::B(ok(tx))
            }
        })
        .then(|_| Ok(()))
}



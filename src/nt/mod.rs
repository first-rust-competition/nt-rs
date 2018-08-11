use std::net::SocketAddr;
use futures::Future;
use futures::future::{ok, Either};
use futures::sync::mpsc::{Sender, Receiver};
use futures::sync::oneshot::{channel, Sender as OneshotSender};
use tokio::prelude::*;
use tokio_core::reactor::Core;

use nt_packet::ClientMessage;
use proto::*;
use proto::types::*;

use std::sync::{Arc, Mutex};
use std::io::Error;
use std::thread;
use std::collections::HashMap;

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
        let (tx, rx) = channel();

        let thread_state = state.clone();
        let _ = thread::spawn(move || {
            let mut core = Core::new().unwrap();
            let handle = core.handle();
            let state = thread_state;
            {
                state.lock().unwrap().set_handle(handle.clone().remote().clone());
            }

            core.run(Connection::new(&handle, &target, client_name, state, tx)).unwrap()
        });

        rx.wait().unwrap();

        {
            while state.lock().unwrap().connection_state().connecting() {

            }
        }

        NetworkTables {
            state,
        }
    }

    pub fn entries(&self) -> HashMap<u16, EntryData> {
        self.state.lock().unwrap().entries()
    }

    pub fn get_entry(&self, id: u16) -> EntryData {
        let state = self.state.lock().unwrap().clone();
        state.get_entry(id).clone()
    }

    pub fn create_entry(&mut self, data: EntryData) {
        self.state.lock().unwrap().create_entry(data);
    }

    pub fn delete_entry(&mut self, id: u16) {
        self.state.lock().unwrap().delete_entry(id);
    }

    pub fn delete_all_entries(&mut self) {
        self.state.lock().unwrap().delete_all_entries();
    }

    pub fn update_entry(&mut self, id: u16, new_value: EntryValue) {
        self.state.lock().unwrap().update_entry(id, new_value);
    }

    pub fn update_entry_flags(&mut self, id: u16, flags: u8) {
        self.state.lock().unwrap().update_entry_flags(id, flags);
    }

    pub fn connected(&self) -> bool {
        self.state.lock().unwrap().connection_state().connected()
    }
}

pub fn send_packets(tx: impl Sink<SinkItem=Box<ClientMessage>, SinkError=Error>, rx: Receiver<Box<ClientMessage>>) -> impl Future<Item=(), Error=()> {
    debug!("Spawned packet send loop");
    rx
        .map_err(|_| ())
        .fold(tx, |tx, packet| tx.send(packet).map_err(|_| ()))
        .then(|_| Ok(()))
}

/// Function containing the future for polling the remote peer for new packets
/// Expects to be started with `tokio::spawn`
/// Mutates the `state` as packets are received
pub fn poll_socket(state: Arc<Mutex<State>>, rx: impl Stream<Item=Packet, Error=Error>, tx: Sender<Box<ClientMessage>>) -> impl Future<Item=(), Error=()> {
    debug!("Spawned socket poll");

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



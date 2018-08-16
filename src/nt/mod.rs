use std::net::SocketAddr;
use futures::Future;
use futures::future::{ok, Either, err};
use futures::sync::mpsc::{Sender, Receiver, channel as mpsc_channel};
use futures::sync::oneshot::channel;
use tokio::prelude::*;
use tokio_core::reactor::Core;

use nt_packet::ClientMessage;
use proto::*;
use proto::types::*;

use std::sync::{Arc, Mutex};
use std::io::{Error, ErrorKind};
use std::thread;
use std::collections::HashMap;

pub(crate) mod state;
mod conn;
mod handler;
/// Module containing definitions for working with NetworkTables entries
mod entry;
mod callback;

use self::handler::*;
use self::state::*;
use self::conn::Connection;

pub use self::entry::*;
pub use self::callback::CallbackType;

/// Core struct representing a connection to a NetworkTables server
pub struct NetworkTables {
    /// Contains the initial connection future with a reference to the framed NT codec
    state: Arc<Mutex<State>>,
    end_tx: Sender<()>
}

impl NetworkTables {
    /// Performs the initial connection process to the given `target`.
    /// Assumes that target is a valid, running NetworkTables server.
    /// Returns a new [`NetworkTables`] once a connection has been established.
    /// If any errors are returned when trying to perform the connection, returns an [`Err`]
    pub fn connect(client_name: &'static str, target: SocketAddr) -> ::Result<NetworkTables> {
        let state = Arc::new(Mutex::new(State::new()));
        state.lock().unwrap().set_connection_state(ConnectionState::Connecting);
        let (tx, rx) = channel();

        let (end_tx, end_rx) = mpsc_channel(1);

        let thread_state = state.clone();
        let _ = thread::spawn(move || {
            let mut core = Core::new().unwrap();
            let handle = core.handle();
            let state = thread_state;
            {
                state.lock().unwrap().set_handle(handle.clone().remote().clone());
            }

            core.run(Connection::new(&handle, &target, client_name, state, tx, end_rx)).unwrap();
        });

        rx.wait()?;

        {
            while state.lock().unwrap().connection_state().connecting() {}
        }

        Ok(NetworkTables {
            state,
            end_tx
        })
    }

    /// Registers the given closure `cb` as a callback to be called for [`CallbackType`] `action`
    /// When `action` occurs due to either network or user events, all callbacks registered for that type will be called
    pub fn add_callback<F>(&mut self, action: CallbackType, cb: F)
        where F: Fn(&EntryData) + Send + 'static
    {
        let mut state = self.state.lock().unwrap();
        state.callbacks_mut().insert(action, Box::new(cb));
    }

    /// Returns a clone of all the entries this client currently knows of.
    pub fn entries(&self) -> HashMap<u16, EntryData> {
        self.state.lock().unwrap().entries()
    }

    /// Returns an [`Entry`] for the given `id`
    /// The underlying value of the entry cannot be mutated.
    pub fn get_entry(&self, id: u16) -> Entry {
        Entry::new(self, id)
    }

    /// Returns an [`EntryMut`] for the given `id`
    /// The underlying value of the entry can be mutated through the given [`EntryMut`]
    pub fn get_entry_mut(&mut self, id: u16) -> EntryMut {
        EntryMut::new(self, id)
    }

    /// Creates a new entry with data contained in `data`.
    /// Returns the id of the new entry, once the server has assigned it
    pub fn create_entry(&mut self, data: EntryData) -> u16 {
        let rx = self.state.lock().unwrap().create_entry(data);
        let id = rx.wait().next().unwrap().unwrap();
        id
    }

    /// Deletes the entry with id `id` from the server the client is currently connected to
    /// Must be used with care. Cannot be undone
    pub(crate) fn delete_entry(&mut self, id: u16) {
        self.state.lock().unwrap().delete_entry(id);
    }

    /// Deletes all entries from the server this client is currently connected to
    /// Must be used with care. Cannot be undone
    pub fn delete_all_entries(&mut self) {
        self.state.lock().unwrap().delete_all_entries();
    }

    /// Updates the value of the entry with id `id`.
    /// The updated value of the entry will match `new_value`
    pub(crate) fn update_entry(&mut self, id: u16, new_value: EntryValue) {
        self.state.lock().unwrap().update_entry(id, new_value);
    }

    /// Updates the flags of the entry with id `id`.
    pub(crate) fn update_entry_flags(&mut self, id: u16, flags: u8) {
        self.state.lock().unwrap().update_entry_flags(id, flags);
    }

    /// Checks if the client is actively connected to an NT server
    /// true if the 3-way handshake has been completed, and the client is fully synchronized
    pub fn connected(&self) -> bool {
        self.state.lock().unwrap().connection_state().connected()
    }
}

impl Drop for NetworkTables {
    fn drop(&mut self) {
        self.end_tx.clone().send(()).wait().unwrap();
    }
}

#[doc(hidden)]
pub(crate) fn send_packets(tx: impl Sink<SinkItem=Box<ClientMessage>, SinkError=Error>, rx: Receiver<Box<ClientMessage>>) -> impl Future<Item=(), Error=()> {
    debug!("Spawned packet send loop");
    rx
        .fold(tx, |tx, packet| tx.send(packet)
            .map_err(|e| error!("Packet sender encountered an error: {}", e)))
        .map(drop)
}

#[doc(hidden)]
pub fn poll_socket(state: Arc<Mutex<State>>, rx: impl Stream<Item=Packet, Error=Error>, tx: Sender<Box<ClientMessage>>, end_rx: Receiver<()>) -> impl Future<Item=(), Error=Error> {
    debug!("Spawned socket poll");

    // This function will be called as new packets arrive. Has to be `fold` to maintain ownership over the write half of our codec
    rx.map(Either::A).select(end_rx.map(Either::B).map_err(|_| Error::new(ErrorKind::Other, "Error encountered from closing channel")))
        .fold(tx, move |tx, packet| {
            match packet {
                Either::A(packet) => match handle_packet(packet, state.clone(), tx.clone()) {
                    Ok(Some(packet)) => Either::A(tx.send(packet).map_err(|e| Error::new(ErrorKind::Other, format!("{}", e)))),
                    Ok(None) => Either::B(ok(tx)),
                    Err(e) => Either::B(err(e)),
                }
                Either::B(_) => Either::B(err(Error::new(ErrorKind::Other, "NetworkTables dropped")))
            }
        })
        .map_err(|e| {
            error!("handle_packet encountered an error: {}", e);
            e
        })
        .map(drop)
}


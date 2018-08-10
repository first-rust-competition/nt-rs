use std::collections::HashMap;
use std::time::{Duration, Instant};

use proto::types::EntryValue;
use proto::KeepAlive;
use nt_packet::ClientMessage;

use futures::{Stream, Sink, Future};
use futures::sync::mpsc::Sender;
use tokio::timer::Interval;
use tokio;

/// Enum representing what part of the connection the given `State` is currently in
pub enum ConnectionState {
    /// Represents a `State` that is not currently connected, or attempting to connect to a NetworkTables server
    Idle,
    /// Represents a `State` that is attempting a connection to a NetworkTables server
    Connecting,
    /// Represents a `State` that has connected to a NetworkTables server
    Connected(Sender<Box<ClientMessage>>),
}

/// Struct containing the state of a connection
/// Passed around the application as necessary
pub struct State {
    /// Represents the current state of the connection
    connection_state: ConnectionState,
    /// Contains the entries received from the server. Updated as they are sent
    entries: HashMap<u16, EntryValue>,
}

impl State {
    /// Creates a new, empty `State`
    pub fn new() -> State {
        State {
            connection_state: ConnectionState::Idle,
            entries: HashMap::new(),
        }
    }

    /// Changes the connection state of `self`
    pub fn set_state(&mut self, state: ConnectionState) {
        self.connection_state = state;

        if let ConnectionState::Connected(ref tx) = self.connection_state {
            debug!("Spawning KeepAlive Looper");
            tokio::spawn(Interval::new(Instant::now(), Duration::from_secs(10))
                .map_err(|_| ())
                .fold(tx.clone(), |tx, _| {
                    debug!("Looping");
                    tx.send(Box::new(KeepAlive)).map_err(|_| ())
                })
                .then(|_| Ok(())));
        }
    }

    /// Adds a NetworkTables entry with the given `key` and `entry`
    /// Called in response to packet 0x10 Entry Assignment
    pub fn add_entry(&mut self, key: u16, entry: EntryValue) {
        debug!("State updated: {:#x} ==> {:?}", key, entry);
        self.entries.insert(key, entry);
    }

    /// Removes a NetworkTables entry of the given `key`
    /// Called in response to packet 0x13 Entry Delete
    pub fn remove_entry(&mut self, key: u16) {
        debug!("State updated: Deleting {:#x}", key);
        self.entries.remove(&key);
    }
}
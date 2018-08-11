use std::collections::HashMap;
use std::time::{Duration, Instant};
use proto::types::{EntryData, EntryValue};
use proto::{KeepAlive, EntryAssignment, EntryDelete};
use nt_packet::ClientMessage;

use futures::{Stream, Sink, Future};
use futures::future::ok;
use futures::sync::mpsc::Sender;
use tokio::timer::Interval;
use tokio;
use tokio::executor::{DefaultExecutor, Executor};
use tokio_core::reactor::Remote;

/// Enum representing what part of the connection the given `State` is currently in
pub enum ConnectionState {
    /// Represents a `State` that is not currently connected, or attempting to connect to a NetworkTables server
    Idle,
    /// Represents a `State` that is attempting a connection to a NetworkTables server
    Connecting,
    /// Represents a `State` that has connected to a NetworkTables server
    Connected(Sender<Box<ClientMessage>>),
}

use std::fmt::{Debug, Formatter, Error};

impl Debug for ConnectionState {
    fn fmt<'a>(&self, f: &mut Formatter<'a>) -> Result<(), Error> {
        match self {
            ConnectionState::Idle => f.write_str("Idle"),
            ConnectionState::Connecting => f.write_str("Connecting"),
            ConnectionState::Connected(_) => f.write_str("Connected"),
        }
    }
}

impl ConnectionState {
    pub fn connected(&self) -> bool {
        match self {
            &ConnectionState::Idle => false,
            _ => true,
        }
    }
}

/// Struct containing the state of a connection
/// Passed around the application as necessary
pub struct State {
    /// Represents the current state of the connection
    connection_state: ConnectionState,
    /// Contains the entries received from the server. Updated as they are sent
    entries: HashMap<u16, EntryData>,
    /// Contains the last sequence number for entries
    last_seqnum: u16,
    handle: Option<Box<Remote>>
}

impl State {
    /// Creates a new, empty `State`
    pub fn new() -> State {
        State {
            connection_state: ConnectionState::Idle,
            entries: HashMap::new(),
            last_seqnum: 0,
            handle: None
        }
    }

    pub fn set_handle(&mut self, handle: Remote) {
        self.handle = Some(Box::new(handle));
    }

    pub fn create_entry(&mut self, data: EntryData) {
        if let ConnectionState::Connected(ref tx) = self.connection_state {
            let tx = tx.clone();
            self.last_seqnum += 1;

            let assignment = EntryAssignment {
                entry_name: data.name,
                entry_type: data.value.entry_type(),
                entry_id: 0xFFFF,
                entry_sequence_num: self.last_seqnum,
                entry_flags: data.flags,
                entry_value: data.value,
            };

            // Unwrap because at this point it's broken if we don't send
            self.handle.clone().unwrap()
                .spawn(|_| ok(assignment).and_then(move |packet| tx.send(Box::new(packet))).then(|_| Ok(())));
        }
    }

    pub fn connection_state(&self) -> &ConnectionState {
        &self.connection_state
    }

    /// Changes the connection state of `self`
    pub fn set_connection_state(&mut self, state: ConnectionState) {
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
    pub fn add_entry(&mut self, info: EntryAssignment) {
        let data = EntryData::new(info.entry_name, info.entry_flags, info.entry_value);

        self.entries.insert(info.entry_id, data);
        self.last_seqnum = info.entry_sequence_num;
    }

    /// Removes a NetworkTables entry of the given `key`
    /// Called in response to packet 0x13 Entry Delete
    pub fn remove_entry(&mut self, key: u16) {
        debug!("State updated: Deleting {:#x}", key);

        self.entries.remove(&key);
    }
}
use std::collections::HashMap;
use proto::types::EntryValue;

/// Enum representing what part of the connection the given `State` is currently in
#[derive(PartialEq, Debug)]
pub enum ConnectionState {
    /// Represents a `State` that is not currently connected, or attempting to connect to a NetworkTables server
    Idle,
    /// Represents a `State` that is attempting a connection to a NetworkTables server
    Connecting,
    /// Represents a `State` that has connected to a NetworkTables server
    Connected,
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
            entries: HashMap::new()
        }
    }

    /// Changes the connection state of `self`
    pub fn set_state(&mut self, state: ConnectionState) {
        println!("State updated: New state {:?}", state);
        self.connection_state = state;
    }

    /// Adds a NetworkTables entry with the given `key` and `entry`
    /// Called in response to packet 0x10 Entry Assignment
    pub fn add_entry(&mut self, key: u16, entry: EntryValue) {
        println!("State updated: {:#x} ==> {:?}", key, entry);
        self.entries.insert(key, entry);
    }

    /// Removes a NetworkTables entry of the given `key`
    /// Called in response to packet 0x13 Entry Delete
    pub fn remove_entry(&mut self, key: u16) {
        println!("State updated: Deleting {:#x}", key);
        self.entries.remove(&key);
    }
}
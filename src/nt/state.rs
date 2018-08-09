use std::collections::HashMap;
use proto::types::EntryValue;

#[derive(PartialEq, Debug)]
pub enum ConnectionState {
    Idle,
    Connecting,
    Connected,
}

pub struct State {
    connection_state: ConnectionState,
    entries: HashMap<u16, EntryValue>,
}

impl State {
    pub fn new() -> State {
        State {
            connection_state: ConnectionState::Idle,
            entries: HashMap::new()
        }
    }

    pub fn set_state(&mut self, state: ConnectionState) {
        println!("State updated: New state {:?}", state);
        self.connection_state = state;
    }

    pub fn add_entry(&mut self, key: u16, entry: EntryValue) {
        println!("State updated: {:#x} ==> {:?}", key, entry);
        self.entries.insert(key, entry);
    }

    pub fn remove_entry(&mut self, key: u16) {
        println!("State updated: Deleting {:#x}", key);
        self.entries.remove(&key);
    }
}
use std::collections::HashMap;
use std::time::{Duration, Instant};
use proto::types::{EntryData, EntryValue};
use proto::*;
use nt_packet::ClientMessage;
use nt::callback::*;

use futures::{Stream, Sink, Future};
use futures::sync::mpsc::{channel, Sender, Receiver};
use tokio::timer::Interval;
use tokio_core::reactor::Remote;

use multimap::MultiMap;

/// Enum representing what part of the connection the given `State` is currently in
#[derive(Clone)]
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
    /// Function used internally to determine whether the client is currently connected to the server
    /// This function returns true if the connection has been established, and the client is synchronized with the server.
    pub(crate) fn connected(&self) -> bool {
        match self {
            &ConnectionState::Connected(_) => true,
            _ => false,
        }
    }

    /// Function used internally to determine whether the client is currently connecting to the server
    /// This function returns true from the time the socket is opened, to the time the 3-way handshake has been completed
    pub(crate) fn connecting(&self) -> bool {
        match self {
            &ConnectionState::Connecting => true,
            _ => false,
        }
    }
}

/// Struct containing the internal state of a connection
//#[derive(Clone)]
pub struct State {
    /// Represents the current state of the connection
    connection_state: ConnectionState,
    /// Contains the entries received from the server. Updated as they are sent
    entries: HashMap<u16, EntryData>,
    handle: Option<Box<Remote>>,
    pending_entries: Vec<(EntryData, Sender<u16>)>,
    callbacks: MultiMap<CallbackType, Box<Action>>,
    rpc_unique_id: u16,
}

impl State {
    /// Creates a new, empty `State`
    pub(crate) fn new() -> State {
        State {
            connection_state: ConnectionState::Idle,
            entries: HashMap::new(),
            handle: None,
            pending_entries: Vec::new(),
            callbacks: MultiMap::new(),
            rpc_unique_id: 0
        }
    }

    #[doc(hidden)]
    pub(crate) fn set_handle(&mut self, handle: Remote) {
        self.handle = Some(Box::new(handle));
    }

    pub(crate) fn call_rpc(&mut self, def: EntryData) {
        if let ConnectionState::Connected(ref tx) = self.connection_state {

        }
    }

    /// Called internally from [`NetworkTables`](struct.NetworkTables.html) to create a new entry on the server.
    /// `self` is updated with the new entry when the server re-broadcasts, having given correct metadata to the item.
    pub(crate) fn create_entry(&mut self, data: EntryData) -> Receiver<u16> {
        if let ConnectionState::Connected(ref tx) = self.connection_state {
            let tx = tx.clone();

            // Gross, but oneshots aren't Clone
            let (notify_tx, rx) = channel(1);
            self.pending_entries.push((data.clone(), notify_tx.clone()));

            let chk_data = data.clone();

            let packet = EntryAssignment {
                entry_name: data.name,
                entry_type: data.value.entry_type(),
                entry_id: 0xFFFF,
                entry_sequence_num: 1,
                entry_flags: data.flags,
                entry_value: data.value,
            };

            if let Some((id, _)) = self.entries.iter().find(|(_, it)| **it == chk_data) {
                let id = id.clone();
                self.handle.clone().unwrap()
                    .spawn(move |_| notify_tx.send(id).map_err(drop)
                        .then(|_| Ok(())));
                return rx;
            }

            // Unwrap because at this point it's broken if we don't send
            self.handle.clone().unwrap().spawn(|_|
                tx.send(Box::new(packet)).then(|_| Ok(())));

            rx
        } else {
            panic!("Cannot create entry while disconnected");
        }
    }

    /// Called internally by [`NetworkTables`](struct.NetworkTables.html) to get the current [`ConnectionState`](enum.ConnectionState.html)
    pub(crate) fn connection_state(&self) -> &ConnectionState {
        &self.connection_state
    }

    /// Called internally to update the [`ConnectionState`](enum.ConnectionState.html) of `self`
    pub(crate) fn set_connection_state(&mut self, state: ConnectionState) {
        self.connection_state = state;

        if let ConnectionState::Connected(ref tx) = self.connection_state {
            let tx = tx.clone();
            debug!("Spawning KeepAlive Looper");
            self.handle.clone().unwrap().spawn(move |_| Interval::new(Instant::now(), Duration::from_secs(10))
                .map_err(|_| ())
                .fold(tx, |tx, _| {
                    debug!("Looping");
                    tx.send(Box::new(KeepAlive)).map_err(|_| ())
                })
                .then(|_| Ok(())));
        }
    }

    /// Adds a NetworkTables entry with the given `key` and `entry`
    /// Called in response to packet 0x10 Entry Assignment
    pub(crate) fn add_entry(&mut self, info: EntryAssignment) {
        if info.entry_value == EntryValue::Pass {
            return;
        }

        let data = EntryData::new_with_seqnum(info.entry_name, info.entry_flags, info.entry_value, info.entry_sequence_num);

        let mut remove_idx = None;
        if let Some((i, (_, tx))) = self.pending_entries.iter().enumerate().find(|(_, (it, _))| *it == data.clone()) {
            remove_idx = Some(i);
            tx.clone().send(info.entry_id).and_then(|mut tx| tx.close()).wait().unwrap();
        }

        if let Some(i) = remove_idx {
            self.pending_entries.remove(i);
        }

        if self.entries.contains_key(&info.entry_id) {
            let entry = &self.entries[&info.entry_id];
            if info.entry_sequence_num != entry.seqnum + 1 {
                return;
            }
        }

        self.callbacks.iter_all().filter(|&(key, _)| *key == CallbackType::Add)
            .flat_map(|(_, cbs)| cbs)
            .for_each(|cb| cb(&data));

        self.entries.insert(info.entry_id, data);
    }

    /// Called internally to update the value of an entry
    pub(crate) fn handle_entry_updated(&mut self, update: EntryUpdate) {
        let old_entry = self.get_entry_mut(update.entry_id);


        if update.entry_sequence_num != old_entry.seqnum + 1 {
            return;
        }

        if update.entry_type == old_entry.value.entry_type() {
            old_entry.value = update.entry_value;
            old_entry.seqnum += 1;

            let old_entry = self.get_entry(update.entry_id);

            self.callbacks.iter_all().filter(|&(key, _)| *key == CallbackType::Update)
                .flat_map(|(_, cbs)| cbs)
                .for_each(|cb| cb(&old_entry))
        }

    }

    /// Removes a NetworkTables entry of the given `key`
    /// Called in response to packet 0x13 Entry Delete
    pub(crate) fn remove_entry(&mut self, key: u16) {
        debug!("State updated: Deleting {:#x}", key);
        let entry_to_delete = &self.entries[&key];

        self.callbacks.iter_all().filter(|&(key, _)| *key == CallbackType::Delete)
            .flat_map(|(_, cbs)| cbs)
            .for_each(|cb| cb(entry_to_delete));

        self.entries.remove(&key);
    }

    /// Called internally to delete the entry with id `key` from this connection
    /// `self` will be updated when the server re-broadcasts the deletion.
    pub(crate) fn delete_entry(&mut self, key: u16) {
        if let ConnectionState::Connected(tx) = self.connection_state.clone() {
            self.entries.remove(&key);
            let delete = EntryDelete::new(key);
            self.handle.clone().unwrap().spawn(move |_|
                tx.send(Box::new(delete)).then(|_| Ok(())));
        }
    }

    /// Called internally to delete all entries from this connection
    /// `self` will be updated when the server re-broadcasts the deletion
    pub(crate) fn delete_all_entries(&mut self) {
        if let ConnectionState::Connected(tx) = self.connection_state.clone() {
            let packet = DeleteAllEntries::new();
            self.handle.clone().unwrap().spawn(move |_|
                tx.send(Box::new(packet)).then(|_| Ok(())))
        }
    }

    /// Called internally to update the value of the entry with id `id`
    /// `self` will be updated with the new value when the server re-broadcasts.
    pub(crate) fn update_entry(&mut self, id: u16, new_value: EntryValue) {
        if let ConnectionState::Connected(tx) = self.connection_state.clone() {
            let entry = &self.entries[&id];
            let packet = EntryUpdate::new(id, entry.seqnum + 1, new_value.entry_type(), new_value);
            self.handle_entry_updated(packet.clone());
            tx.send(Box::new(packet)).wait().unwrap();
        }
    }

    /// Called internally to update the flags of the entry with id `id`
    /// `self` will be updated with the new flags when the server re-broadcasts the update.
    pub(crate) fn update_entry_flags(&mut self, id: u16, flags: u8) {
        if let ConnectionState::Connected(tx) = self.connection_state.clone() {
            let packet = EntryFlagsUpdate::new(id, flags);
            self.handle.clone().unwrap().spawn(move |_|
                tx.send(Box::new(packet)).then(|_| Ok(())));
        }
    }

    pub(crate) fn callbacks_mut(&mut self) -> &mut MultiMap<CallbackType, Box<Action>> {
        &mut self.callbacks
    }

    /// Gets an immutable reference to the entry with id `id` from the internal map of `self`
    pub(crate) fn get_entry(&self, id: u16) -> &EntryData {
        &self.entries[&id]
    }

    /// Gets a mutable reference to the entry with id `id` from the internal map of `self`
    pub(crate) fn get_entry_mut(&mut self, id: u16) -> &mut EntryData {
        self.entries.get_mut(&id).unwrap()
    }

    /// Gets a clone of the internal map of entries held by `self`
    pub(crate) fn entries(&self) -> HashMap<u16, EntryData> {
        self.entries.clone()
    }

    /// Gets a mutable reference of the internal map of entries held by `self`
    pub(crate) fn entries_mut(&mut self) -> &mut HashMap<u16, EntryData> {
        &mut self.entries
    }
}
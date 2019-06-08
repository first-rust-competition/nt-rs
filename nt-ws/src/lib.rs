use std::rc::Rc;
use std::sync::Mutex;
use nt_network::types::EntryValue;
use std::collections::HashMap;

mod framed;
pub mod entry;
mod state;
pub mod callback;

use self::state::WsState;
use crate::entry::{EntryData, Entry};
use crate::callback::CallbackType;
use futures::channel::mpsc::unbounded;
use futures::prelude::*;
use futures::Future;

mod ffi;

/// Struct representing a connection to a remote NT server over websockets
#[derive(Clone)]
pub struct NetworkTables {
    state: Rc<Mutex<WsState>>,
}

impl NetworkTables {
    /// Connects to the server at `ip` with name `client_name`.
    /// Returns a Future resolving to NetworkTables. This can be driven to completion using the stdweb futures executor.
    pub fn connect(ip: &str, client_name: &str) -> impl Future<Output=NetworkTables> {
        let (tx, rx) = unbounded::<()>();
        let _self = NetworkTables { state: WsState::new(ip.to_string(), client_name.to_string(), tx) };

        rx.into_future()
            .then(move |_| futures::future::ready(_self))
    }

    pub fn get_entry(&self, id: u16) -> Entry {
        Entry::new(self, id)
    }

    /// Creates a new entry with the given entry data
    /// Returns a future resolving to the id of the entry. This can be driven to completion using the stdweb futures executor
    pub fn create_entry(&self, data: EntryData) -> impl Future<Output=u16> {
        self.state.lock().unwrap().create_entry(data).into_future().map(|(id, _)| id.unwrap_or(0))
    }

    /// Deletes the entry with the given `id`
    pub fn delete_entry(&self, id: u16) {
        self.state.lock().unwrap().delete_entry(id);
    }

    /// Clears all the entries on the server
    pub fn clear_entries(&self) {
        self.state.lock().unwrap().clear_entries();
    }

    /// Updates the entry associated with `id` with the new value
    pub fn update_entry(&self, id: u16, new_value: EntryValue) {
        self.state.lock().unwrap().update_entry(id, new_value);
    }

    /// Updates the flags associated with the given entry
    pub fn update_entry_flags(&self, id: u16, flags: u8) {
        self.state.lock().unwrap().update_entry_flags(id, flags);
    }

    /// Returns a clone of the entries the client currently knows aout
    pub fn entries(&self) -> HashMap<u16, EntryData> {
        self.state.lock().unwrap().entries().clone()
    }

    /// Registers a callback
    ///
    /// Callbacks will be called when the given `CallbackType` occurs. For example, all callbacks of type
    /// `CallbackType::Add` will be called when the client is informed of a newly created value
    pub fn add_callback(&self, ty: CallbackType, cb: impl FnMut(&EntryData) + 'static) {
        self.state.lock().unwrap().add_callback(ty, Box::new(cb))
    }
}

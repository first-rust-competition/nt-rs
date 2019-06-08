pub mod entry;
pub mod callback;

use crate::Result;

pub use self::entry::*;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use nt_network::types::EntryValue;
use crate::nt::callback::*;
use crate::proto::{State, NTBackend, Client, Server, ClientState, ServerState};
use futures::sync::mpsc::{Sender, channel};
use futures::Future;
use futures::sink::Sink;
use std::net::SocketAddr;

/// Core struct representing a connection to a NetworkTables server
pub struct NetworkTables<T: NTBackend> {
    state: Arc<Mutex<T::State>>,
    close_tx: Sender<()>,
}

impl NetworkTables<Client> {
    /// Connects over TCP to the given ip, with the given client_name
    ///
    /// This call will block the thread until the client has completed the handshake with the server,
    /// at which point the connection will be valid to send and receive data over
    pub fn connect(ip: &str, client_name: &str) -> Result<NetworkTables<Client>> {
        let (close_tx, close_rx) = channel::<()>(1);
        let state = ClientState::new(ip.to_string(), client_name.to_string(), close_rx)?;
        Ok(NetworkTables { state, close_tx })
    }

    /// Connects over websockets to the given ip, with the given client name
    ///
    /// This call will block the thread until the client has completed the handshake with the server,
    /// at which point the connection will be valid to send and receive data over
    #[cfg(feature = "websocket")]
    pub fn connect_ws(ip: &str, client_name: &str) -> Result<NetworkTables<Client>> {
        let (close_tx, close_rx) = channel::<()>(1);
        let state = ClientState::new_ws(ip.to_string(), client_name.to_string(), close_rx)?;

        Ok(NetworkTables { state, close_tx })
    }
}

impl NetworkTables<Server> {
    /// Initializes an NT server over TCP and binds it to the given ip, with the given server name.
    pub fn bind(ip: &str, server_name: &str) -> NetworkTables<Server> {
        let (close_tx, close_rx) = channel::<()>(1);
        let state = ServerState::new(ip.to_string(), server_name.to_string(), close_rx);
        NetworkTables { state, close_tx }
    }

    /// Initializes an NT server over websockets and binds it to the given ip, with the given server name
    #[cfg(feature = "websocket")]
    pub fn bind_ws(ip: &str, server_name: &str) -> NetworkTables<Server> {
        let (close_tx, close_rx) = channel::<()>(1);
        let state = ServerState::new_ws(ip.to_string(), server_name.to_string(), close_rx);
        NetworkTables { state, close_tx }
    }

    /// Adds a callback for connection state updates regarding clients.
    ///
    /// Depending on the chosen callback type, the callback will be called when a new client connects,
    /// or when an existing client disconnects from the server
    pub fn add_connection_callback(&mut self, callback_type: ServerCallbackType, action: impl FnMut(&SocketAddr) + Send + 'static) {
        self.state.lock().unwrap().add_server_callback(callback_type, action);
    }
}

impl<T: NTBackend> NetworkTables<T> {
    /// Returns a copy of the entries recognizes by the connection
    pub fn entries(&self) -> HashMap<u16, EntryData> {
        self.state.lock().unwrap().entries().clone()
    }

    /// Gets the entry with the given id, returning an `Entry` for the specified data
    pub fn get_entry(&self, id: u16) -> Entry<T> {
        Entry::new(self, id)
    }

    /// Creates a new entry with the specified data, returning the id assigned to it by the server
    /// This call may block if this connection is acting as a client, while it waits for the id to be assigned by the remote server
    pub fn create_entry(&self, data: EntryData) -> u16 {
        let id_rx = self.state.lock().unwrap().create_entry(data);
        id_rx.recv().unwrap()
    }

    /// Deletes the entry with the given id
    pub fn delete_entry(&self, id: u16) {
        self.state.lock().unwrap().delete_entry(id);
    }

    /// Clears all the entries associated with this connection
    pub fn clear_entries(&self) {
        self.state.lock().unwrap().clear_entries();
    }

    /// Updates the entry of the given id, with the new value
    pub fn update_entry(&self, id: u16, new_value: EntryValue) {
        self.state.lock().unwrap().update_entry(id, new_value);
    }

    /// Adds an entry callback of the given type.
    ///
    /// Depending on what is chosen, the callback will be notified when a new entry is created,
    /// an existing entry is updated, or an existing entry is deleted.
    pub fn add_callback<F>(&mut self, action: CallbackType, cb: F)
        where F: FnMut(&EntryData) + Send + 'static
    {
        let mut state = self.state.lock().unwrap();
        state.add_callback(action, cb);
    }

    /// Updates the flags associated with the entry of the given id
    pub fn update_entry_flags(&self, id: u16, new_flags: u8) {
        self.state.lock().unwrap().update_entry_flags(id, new_flags);
    }
}

impl<T: NTBackend> Drop for NetworkTables<T> {
    fn drop(&mut self) {
        self.close_tx.clone().send(()).wait().unwrap();
    }
}
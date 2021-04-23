pub mod callback;
pub mod entry;

use crate::Result;

pub use self::entry::*;
use crate::nt::callback::*;
use crate::proto::server::ServerState;
use crate::proto::{client::ClientState, Client, NTBackend, Server, State};
use futures_channel::mpsc::{channel, unbounded, Sender};
use futures_util::StreamExt;
use nt_network::types::EntryValue;
use nt_network::Packet;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::panic::RefUnwindSafe;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;

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
    pub async fn connect(ip: &str, client_name: &str) -> Result<NetworkTables<Client>> {
        let (close_tx, close_rx) = channel::<()>(1);
        let state = ClientState::new(ip.to_string(), client_name.to_string(), close_rx).await?;
        Ok(NetworkTables { state, close_tx })
    }

    /// Attempts to reconnect to the NetworkTables server if the connection had been terminated.
    ///
    /// This function should _only_ be called if you are certain that the previous connection is dead.
    /// Connection status can be determined using callbacks specified with `add_connection_callback`.
    pub async fn reconnect(&mut self) {
        let rt_state = self.state.clone();

        let (close_tx, close_rx) = channel::<()>(1);
        let (packet_tx, packet_rx) = unbounded::<Box<dyn Packet>>();
        let (ready_tx, mut ready_rx) = unbounded();

        self.close_tx = close_tx;
        {
            let mut state = self.state.lock().unwrap();
            state.packet_tx = packet_tx;
            state.entries_mut().clear();
            state.pending_entries.clear();
        }
        thread::spawn(move || {
            let mut rt = Runtime::new().unwrap();
            rt.block_on(crate::proto::client::conn::connection(
                rt_state, packet_rx, ready_tx, close_rx,
            ))
            .unwrap();
        });

        let _ = ready_rx.next().await;
    }

    /// Connects over websockets to the given ip, with the given client name
    ///
    /// This call will block the thread until the client has completed the handshake with the server,
    /// at which point the connection will be valid to send and receive data over
    #[cfg(feature = "websocket")]
    pub async fn connect_ws(ip: &str, client_name: &str) -> Result<NetworkTables<Client>> {
        let (close_tx, close_rx) = channel::<()>(1);
        let state = ClientState::new_ws(ip.to_string(), client_name.to_string(), close_rx).await?;

        Ok(NetworkTables { state, close_tx })
    }

    /// Attempts to reconnect over websockets to the NetworkTables instance.
    ///
    /// This function should _only_ be called if you are certain that the previous connection is dead.
    /// Connection status can be determined using callbacks specified with `add_connection_callback`.
    #[cfg(feature = "websocket")]
    pub async fn reconnect_ws(&mut self) {
        let rt_state = self.state.clone();

        let (close_tx, close_rx) = channel::<()>(1);
        let (packet_tx, packet_rx) = unbounded::<Box<dyn Packet>>();
        let (ready_tx, mut ready_rx) = unbounded();

        self.close_tx = close_tx;
        {
            let mut state = self.state.lock().unwrap();
            state.packet_tx = packet_tx;
            state.entries_mut().clear();
            state.pending_entries.clear();
        }
        thread::spawn(move || {
            let mut rt = Runtime::new().unwrap();
            rt.block_on(crate::proto::client::conn::connection_ws(
                rt_state, packet_rx, ready_tx, close_rx,
            ))
            .unwrap();
        });

        let _ = ready_rx.next().await;
    }

    pub fn add_connection_callback(
        &self,
        callback_type: ConnectionCallbackType,
        action: impl FnMut(&SocketAddr) + Send + 'static,
    ) {
        self.state
            .lock()
            .unwrap()
            .add_connection_callback(callback_type, action);
    }

    pub fn call_rpc(
        &self,
        id: u16,
        parameter: Vec<u8>,
        callback: impl Fn(Vec<u8>) + Send + 'static,
    ) {
        self.state.lock().unwrap().call_rpc(id, parameter, callback);
    }
}

impl NetworkTables<Server> {
    /// Initializes an NT server over TCP and binds it to the given ip, with the given server name.
    pub fn bind(ip: &str, server_name: &str) -> NetworkTables<Server> {
        let (close_tx, close_rx) = channel::<()>(1);
        let state = ServerState::new(ip.to_string(), server_name.to_string(), close_rx);
        NetworkTables { state, close_tx }
    }

    /// Adds a callback for connection state updates regarding clients.
    ///
    /// Depending on the chosen callback type, the callback will be called when a new client connects,
    /// or when an existing client disconnects from the server
    pub fn add_connection_callback(
        &mut self,
        callback_type: ConnectionCallbackType,
        action: impl FnMut(&SocketAddr) + Send + 'static,
    ) {
        self.state
            .lock()
            .unwrap()
            .add_server_callback(callback_type, action);
    }

    pub fn create_rpc(
        &mut self,
        data: EntryData,
        callback: impl Fn(Vec<u8>) -> Vec<u8> + Send + Sync + RefUnwindSafe + 'static,
    ) {
        self.state.lock().unwrap().create_rpc(data, callback);
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
    pub async fn create_entry(&self, data: EntryData) -> crate::Result<u16> {
        let mut rx = self.state.lock().unwrap().create_entry(data)?;
        Ok(rx.next().await.unwrap())
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
    where
        F: FnMut(&EntryData) + Send + 'static,
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
        let _ = self.close_tx.try_send(());
    }
}

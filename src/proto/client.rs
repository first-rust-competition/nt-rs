use super::State;
use crate::{
    Action, CallbackType, ConnectionAction, ConnectionCallbackType, EntryData, EntryValue,
    RpcCallback,
};
use futures_channel::mpsc::{channel, unbounded, Receiver, Sender, UnboundedSender};
use futures_util::StreamExt;
use multimap::MultiMap;
use nt_network::{
    ClearAllEntries, EntryAssignment, EntryDelete, EntryFlagsUpdate, EntryUpdate, Packet,
    RpcExecute,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;

pub(crate) mod conn;

pub struct ClientState {
    ip: String,
    name: String,
    entries: HashMap<u16, EntryData>,
    callbacks: MultiMap<CallbackType, Box<Action>>,
    connection_callbacks: MultiMap<ConnectionCallbackType, Box<ConnectionAction>>,
    pub(crate) pending_entries: HashMap<String, Sender<u16>>,
    pub(crate) packet_tx: UnboundedSender<Box<dyn Packet>>,
    rpc_callbacks: HashMap<u16, Box<RpcCallback>>,
    next_rpc_id: u16,
}

impl ClientState {
    pub async fn new(ip: String, name: String, close_rx: Receiver<()>) -> Arc<Mutex<ClientState>> {
        let (packet_tx, packet_rx) = unbounded::<Box<dyn Packet>>();
        let (ready_tx, mut ready_rx) = unbounded::<()>();

        let state = Arc::new(Mutex::new(ClientState {
            ip,
            name,
            entries: HashMap::new(),
            callbacks: MultiMap::new(),
            connection_callbacks: MultiMap::new(),
            pending_entries: HashMap::new(),
            packet_tx,
            rpc_callbacks: HashMap::new(),
            next_rpc_id: 0,
        }));

        let rt_state = state.clone();
        thread::spawn(move || {
            let mut rt = Runtime::new().unwrap();
            rt.block_on(conn::connection(rt_state, packet_rx, ready_tx, close_rx))
                .unwrap();
        });

        ready_rx.next().await;
        state
    }

    #[cfg(feature = "websocket")]
    pub async fn new_ws(
        url: String,
        name: String,
        close_rx: Receiver<()>,
    ) -> Arc<Mutex<ClientState>> {
        let (packet_tx, packet_rx) = unbounded::<Box<dyn Packet>>();
        let (ready_tx, mut ready_rx) = unbounded::<()>();

        let state = Arc::new(Mutex::new(ClientState {
            ip: url,
            name,
            entries: HashMap::new(),
            callbacks: MultiMap::new(),
            connection_callbacks: MultiMap::new(),
            pending_entries: HashMap::new(),
            packet_tx,
            rpc_callbacks: HashMap::new(),
            next_rpc_id: 0,
        }));

        let rt_state = state.clone();
        thread::spawn(move || {
            let mut rt = Runtime::new().unwrap();

            let _ = rt.block_on(conn::connection_ws(rt_state, packet_rx, ready_tx, close_rx));
        });

        ready_rx.next().await;
        state
    }

    pub fn add_connection_callback(
        &mut self,
        callback_type: ConnectionCallbackType,
        action: impl FnMut(&SocketAddr) + Send + 'static,
    ) {
        self.connection_callbacks
            .insert(callback_type, Box::new(action));
    }

    pub fn call_rpc(
        &mut self,
        id: u16,
        parameter: Vec<u8>,
        callback: impl Fn(Vec<u8>) + Send + 'static,
    ) {
        self.rpc_callbacks
            .insert(self.next_rpc_id, Box::new(callback));
        self.packet_tx
            .unbounded_send(Box::new(RpcExecute::new(id, self.next_rpc_id, parameter)))
            .unwrap();

        self.next_rpc_id += 1;
    }
}

impl State for ClientState {
    fn entries(&self) -> &HashMap<u16, EntryData> {
        &self.entries
    }

    fn entries_mut(&mut self) -> &mut HashMap<u16, EntryData> {
        &mut self.entries
    }

    fn create_entry(&mut self, data: EntryData) -> Receiver<u16> {
        let (tx, rx) = channel::<u16>(1);
        self.pending_entries.insert(data.name.clone(), tx);
        self.packet_tx
            .unbounded_send(Box::new(EntryAssignment::new(
                data.name.clone(),
                data.entry_type(),
                0xFFFF,
                data.seqnum,
                data.flags,
                data.value,
            )))
            .unwrap();
        rx
    }

    fn delete_entry(&mut self, id: u16) {
        let packet = EntryDelete::new(id);
        self.packet_tx.unbounded_send(Box::new(packet)).unwrap();
    }

    fn update_entry(&mut self, id: u16, new_value: EntryValue) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.value = new_value.clone();
            self.packet_tx
                .unbounded_send(Box::new(EntryUpdate::new(
                    id,
                    entry.seqnum + 1,
                    entry.entry_type(),
                    new_value,
                )))
                .unwrap();
        }
    }

    fn update_entry_flags(&mut self, id: u16, flags: u8) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.flags = flags;
            self.packet_tx
                .unbounded_send(Box::new(EntryFlagsUpdate::new(id, flags)))
                .unwrap();
        }
    }

    fn clear_entries(&mut self) {
        self.packet_tx
            .unbounded_send(Box::new(ClearAllEntries::new()))
            .unwrap();
        self.entries.clear();
    }

    fn add_callback(
        &mut self,
        callback_type: CallbackType,
        action: impl FnMut(&EntryData) + Send + 'static,
    ) {
        self.callbacks.insert(callback_type, Box::new(action));
    }
}

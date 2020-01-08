use crate::proto::State;
use crate::{
    Action, CallbackType, ConnectionAction, ConnectionCallbackType, EntryData, EntryValue,
};
use futures_channel::mpsc::{channel, Receiver, UnboundedSender};
use multimap::MultiMap;
use nt_network::{
    ClearAllEntries, EntryAssignment, EntryDelete, EntryFlagsUpdate, EntryUpdate, Packet,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;

mod conn;

pub struct ServerState {
    server_name: String,
    clients: HashMap<SocketAddr, UnboundedSender<Box<dyn Packet>>>,
    entries: HashMap<u16, EntryData>,
    callbacks: MultiMap<CallbackType, Box<Action>>,
    server_callbacks: MultiMap<ConnectionCallbackType, Box<ConnectionAction>>,
    next_id: u16,
}

fn spawn_rt(ip: String, state: Arc<Mutex<ServerState>>, close_rx: Receiver<()>) {
    thread::spawn(move || {
        let mut rt = Runtime::new().unwrap();
        rt.block_on(conn::connection(ip, state, close_rx)).unwrap();
    });
}

impl ServerState {
    pub fn new(ip: String, server_name: String, close_rx: Receiver<()>) -> Arc<Mutex<ServerState>> {
        let state = Arc::new(Mutex::new(ServerState {
            server_name,
            clients: HashMap::new(),
            entries: HashMap::new(),
            callbacks: MultiMap::new(),
            server_callbacks: MultiMap::new(),
            next_id: 0,
        }));

        let rt_state = state.clone();
        spawn_rt(ip, rt_state, close_rx);

        state
    }

    pub fn add_server_callback(
        &mut self,
        callback_type: ConnectionCallbackType,
        action: impl FnMut(&SocketAddr) + Send + 'static,
    ) {
        self.server_callbacks
            .insert(callback_type, Box::new(action));
    }
}

impl State for ServerState {
    fn entries(&self) -> &HashMap<u16, EntryData> {
        &self.entries
    }

    fn entries_mut(&mut self) -> &mut HashMap<u16, EntryData> {
        &mut self.entries
    }

    fn create_entry(&mut self, data: EntryData) -> Receiver<u16> {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.insert(id, data.clone());

        let packet = Box::new(EntryAssignment::new(
            data.name.clone(),
            data.entry_type(),
            id,
            data.seqnum,
            data.flags,
            data.value.clone(),
        ));
        for tx in self.clients.values() {
            tx.unbounded_send(packet.clone()).unwrap()
        }

        self.callbacks
            .iter_all_mut()
            .filter(|(cb, _)| **cb == CallbackType::Add)
            .flat_map(|(_, cbs)| cbs)
            .for_each(|cb| cb(&data));

        let (mut tx, rx) = channel(1);
        tx.try_send(id).unwrap();
        rx
    }

    fn delete_entry(&mut self, id: u16) {
        let entry = self.entries.remove(&id).unwrap();

        let packet = Box::new(EntryDelete::new(id));
        for tx in self.clients.values() {
            tx.unbounded_send(packet.clone()).unwrap();
        }

        self.callbacks
            .iter_all_mut()
            .filter(|(cb, _)| **cb == CallbackType::Delete)
            .flat_map(|(_, cbs)| cbs)
            .for_each(|cb| cb(&entry));
    }

    fn update_entry(&mut self, id: u16, new_value: EntryValue) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.seqnum = entry.seqnum.wrapping_add(1);
            entry.value = new_value;

            let packet = Box::new(EntryUpdate::new(
                id,
                entry.seqnum,
                entry.entry_type(),
                entry.value.clone(),
            ));
            for tx in self.clients.values() {
                tx.unbounded_send(packet.clone()).unwrap();
            }

            let entry = &*entry;

            self.callbacks
                .iter_all_mut()
                .filter(|(cb, _)| **cb == CallbackType::Update)
                .flat_map(|(_, cbs)| cbs)
                .for_each(|cb| cb(entry));
        }
    }

    fn update_entry_flags(&mut self, id: u16, flags: u8) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.flags = flags;

            let packet = Box::new(EntryFlagsUpdate::new(id, flags));
            for tx in self.clients.values() {
                tx.unbounded_send(packet.clone()).unwrap();
            }
        }
    }

    fn clear_entries(&mut self) {
        self.entries.clear();

        let packet = Box::new(ClearAllEntries::new());
        for tx in self.clients.values() {
            tx.unbounded_send(packet.clone()).unwrap();
        }
    }

    fn add_callback(
        &mut self,
        callback_type: CallbackType,
        action: impl FnMut(&EntryData) + Send + 'static,
    ) {
        self.callbacks.insert(callback_type, Box::new(action));
    }
}

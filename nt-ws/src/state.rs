use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;
use stdweb::web::WebSocket;
use nt_network::{ReceivedPacket, NTVersion, ClientHello, EntryAssignment, EntryDelete, EntryUpdate, EntryFlagsUpdate, ClearAllEntries};
use stdweb::web::event::SocketOpenEvent;
use stdweb::web::IEventTarget;
use futures::channel::mpsc::{self, UnboundedSender, UnboundedReceiver};

use crate::framed::WsFramed;
use crate::entry::EntryData;
use crate::callback::*;
use nt_network::types::EntryValue;
use multimap::MultiMap;

pub struct WsState {
    entries: HashMap<u16, EntryData>,
    pending_entries: HashMap<String, UnboundedSender<u16>>,
    callbacks: MultiMap<CallbackType, Box<Action>>,
    conn: Option<Rc<Mutex<WsFramed>>>,
}

impl WsState {
    pub fn new(ip: String, client_name: String, notify_tx: mpsc::UnboundedSender<()>) -> Rc<Mutex<WsState>> {
        let mut _self = Rc::new(Mutex::new(WsState { entries: HashMap::new(), pending_entries: HashMap::new(), conn: None, callbacks: MultiMap::new() }));

        let state = _self.clone();

        let conn = WsFramed::new(notify_tx, WebSocket::new_with_protocols(&ip, &["NetworkTables"]).unwrap(),
                                 move |packet| {
                                     match packet {
                                         ReceivedPacket::EntryAssignment(ea) => {
                                             let data = EntryData::new_with_seqnum(ea.entry_name.clone(), ea.entry_flags, ea.entry_value.clone(), ea.entry_seqnum);

                                             let mut state = state.lock().unwrap();
                                             if state.pending_entries.contains_key(&ea.entry_name) {
                                                 state.pending_entries.remove(&ea.entry_name).unwrap().unbounded_send(ea.entry_id).unwrap();
                                             }
                                             state.callbacks.iter_all_mut()
                                                 .filter(|(ty, _)| **ty == CallbackType::Add)
                                                 .flat_map(|(_, cbs)| cbs)
                                                 .for_each(|cb| cb(&data));

                                             state.entries.insert(ea.entry_id, data);
                                         }
                                         ReceivedPacket::EntryDelete(ed) => {
                                             let mut state = state.lock().unwrap();
                                             if let Some(data) = state.entries.remove(&ed.entry_id) {
                                                 state.callbacks.iter_all_mut()
                                                     .filter(|(ty, _)| **ty == CallbackType::Delete)
                                                     .flat_map(|(_, cbs)| cbs)
                                                     .for_each(|cb| cb(&data));
                                             }
                                         }
                                         ReceivedPacket::EntryUpdate(eu) => {
                                             let mut state = state.lock().unwrap();
                                             if let Some(entry) = state.entries.get_mut(&eu.entry_id) {
                                                 if entry.seqnum < eu.entry_seqnum && entry.entry_type() == eu.entry_type {
                                                     entry.seqnum = eu.entry_seqnum;
                                                     entry.value = eu.entry_value.clone();
                                                 }
                                             }
                                             // Gross but we cant have multiple mutable borrows
                                             if let Some(entry) = state.entries.get(&eu.entry_id).cloned() {
                                                 state.callbacks.iter_all_mut()
                                                     .filter(|(ty, _)| **ty == CallbackType::Update)
                                                     .flat_map(|(_, cbs)| cbs)
                                                     .for_each(|cb| cb(&entry));
                                             }
                                         }
                                         ReceivedPacket::EntryFlagsUpdate(efu) => {
                                             let mut state = state.lock().unwrap();
                                             if let Some(entry) = state.entries.get_mut(&efu.entry_id) {
                                                 entry.flags = efu.entry_flags;
                                             }
                                         }
                                         ReceivedPacket::ClearAllEntries(cea) => {
                                             if cea.is_valid() {
                                                 state.lock().unwrap().entries.clear();
                                             }
                                         }
                                         _ => {}
                                     }
                                 });

        let state = conn.clone();
        conn.lock().unwrap().socket().add_event_listener(move |_: SocketOpenEvent| {
            let client_name = client_name.clone();
            let state = state.lock().unwrap();
            state.send(Box::new(ClientHello::new(NTVersion::V3, client_name)));
        });

        _self.lock().unwrap().conn = Some(conn);

        _self
    }

    pub fn create_entry(&mut self, data: EntryData) -> UnboundedReceiver<u16> {
        let (tx, rx) = mpsc::unbounded();
        self.pending_entries.insert(data.name.clone(), tx);
        self.conn.as_ref().expect("Option is None").lock().expect("Failed to lock connection mux").send(Box::new(EntryAssignment::new(data.name.clone(), data.entry_type(), 0xFFFF, data.seqnum, data.flags, data.value)));

        rx
    }

    pub fn delete_entry(&mut self, id: u16) {
        if self.entries.contains_key(&id) {
            self.entries.remove(&id);
            self.conn.as_ref().unwrap().lock().unwrap().send(Box::new(EntryDelete::new(id)));
        }
    }

    pub fn update_entry(&mut self, id: u16, new_value: EntryValue) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.value = new_value;
            entry.seqnum += 1;
            self.conn.as_ref().unwrap().lock().unwrap().send(Box::new(EntryUpdate::new(id, entry.seqnum, entry.entry_type(), entry.value.clone())))
        }
    }

    pub fn update_entry_flags(&mut self, id: u16, flags: u8) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.flags = flags;
            self.conn.as_ref().unwrap().lock().unwrap().send(Box::new(EntryFlagsUpdate::new(id, flags)));
        }
    }

    pub fn clear_entries(&mut self) {
        self.entries.clear();
        self.conn.as_ref().unwrap().lock().unwrap().send(Box::new(ClearAllEntries::new()));
    }

    pub fn entries(&self) -> &HashMap<u16, EntryData> {
        &self.entries
    }

    pub fn add_callback(&mut self, ty: CallbackType, callback: Box<dyn FnMut(&EntryData) + 'static>) {
        self.callbacks.insert(ty, callback);
    }
}

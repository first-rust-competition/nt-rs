use std::sync::{Arc, Mutex};

use futures::sync::mpsc::Sender;
use futures::sync::oneshot::Sender as OneshotSender;

use nt::state::*;
use nt_packet::ClientMessage;
use proto::Packet;
use proto::client::*;

pub fn handle_packet(packet: Packet, state: Arc<Mutex<State>>, tx: Sender<Box<ClientMessage>>) -> Option<Box<ClientMessage>> {
    match packet {
        Packet::ServerHello(packet) => {
            debug!("Got server hello: {:?}", packet);
            None
        }
        Packet::ServerHelloComplete(_) => {
            debug!("Got server hello complete");
            state.lock().unwrap().set_connection_state(ConnectionState::Connected(tx.clone()));
            debug!("Sent ClientHelloComplete");
            Some(Box::new(ClientHelloComplete))
        }
        Packet::ProtocolVersionUnsupported(packet) => {
            println!("Got this {:?}", packet);

            None
        }
        Packet::EntryAssignment(entry /* heheheh */) => {
            let mut state = state.lock().unwrap();

            state.add_entry(entry);
            None
        }
        Packet::EntryDelete(delet) => {
            let mut state = state.lock().unwrap();
            state.remove_entry(delet.entry_id);

            None
        }
        Packet::DeleteAllEntries(delet) => {
            if delet.magic == 0xD06CB27A {
                let mut state = state.lock().unwrap();
                state.entries_mut().clear()
            }

            None
        }
        Packet::EntryUpdate(update) => {
            let mut state = state.lock().unwrap();
            state.update_seqnum(update.entry_sequence_num);
            let old_entry = state.get_entry_mut(update.entry_id);
            if update.entry_type == old_entry.value.entry_type() {
                old_entry.value = update.entry_value;
            }

            None
        }
        Packet::EntryFlagsUpdate(update) => {
            let mut state = state.lock().unwrap();
            let entry = state.get_entry_mut(update.entry_id);
            entry.flags = update.entry_flags;

            None
        }
        _ => None
    }
}

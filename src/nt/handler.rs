use std::sync::{Arc, Mutex};

use nt::state::*;
use nt_packet::ClientMessage;
use proto::Packet;
use proto::client::*;

pub fn handle_packet(packet: Packet, state: Arc<Mutex<State>>) -> Option<Box<ClientMessage>> {
    match packet {
        Packet::ServerHello(packet) => {
            println!("Got server hello: {:?}", packet);
            None
        }
        Packet::ServerHelloComplete(_) => {
            println!("Got server hello complete");
            state.lock().unwrap().set_state(ConnectionState::Connected);
            println!("Sent ClientHelloComplete");
            Some(Box::new(ClientHelloComplete))
        }
        Packet::ProtocolVersionUnsupported(packet) => {
            println!("Got this {:?}", packet);

            None
        }
        Packet::EntryAssignment(entry /* heheheh */) => {
            let mut state = state.lock().unwrap();
            state.add_entry(entry.entry_id, entry.entry_value);

            None
        }
        Packet::EntryDelete(delet) => {
            let mut state = state.lock().unwrap();
            state.remove_entry(delet.entry_id);

            None
        }
        _ => None
    }
}
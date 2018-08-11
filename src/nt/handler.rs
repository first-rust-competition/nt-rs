use std::sync::{Arc, Mutex};

use futures::sync::mpsc::Sender;

use nt::state::*;
use nt_packet::ClientMessage;
use proto::Packet;
use proto::client::*;
use proto::*;
use proto::types::*;

pub fn handle_packet(packet: Packet, state: Arc<Mutex<State>>, tx: Sender<Box<ClientMessage>>) -> Option<Box<ClientMessage>> {
    match packet {
        Packet::ServerHello(packet) => {
            debug!("Got server hello: {:?}", packet);
            None
        }
        Packet::ServerHelloComplete(_) => {
            debug!("Got server hello complete");
            state.lock().unwrap().set_connection_state(ConnectionState::Connected(tx));
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
        _ => None
    }
}

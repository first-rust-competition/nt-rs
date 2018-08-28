use std::sync::{Arc, Mutex};
use std::io::{Error, ErrorKind};

use futures::sync::mpsc::Sender;

use nt::state::*;
use nt_packet::ClientMessage;
use proto::Packet;
use proto::client::*;

pub fn handle_packet(packet: Packet, state: Arc<Mutex<State>>, tx: Sender<Box<ClientMessage>>) -> Result<Option<Box<ClientMessage>>, Error> {
    match packet {
        Packet::ServerHello(packet) => {
            debug!("Got server hello: {:?}", packet);
            Ok(None)
        }
        Packet::ServerHelloComplete(_) => {
            debug!("Got server hello complete");
            state.lock().unwrap().set_connection_state(ConnectionState::Connected(tx.clone()));
            debug!("Sent ClientHelloComplete");
            Ok(Some(Box::new(ClientHelloComplete)))
        }
        Packet::ProtocolVersionUnsupported(packet) => {
            debug!("Got this {:?}", packet);

            Err(Error::new(ErrorKind::Other, "Client encountered an error: Protocol version unsupported."))
        }
        Packet::EntryAssignment(entry) => {
            let mut state = state.lock().unwrap();

            state.add_entry(entry);
            Ok(None)
        }
        Packet::EntryDelete(delet) => {
            let mut state = state.lock().unwrap();
            state.remove_entry(delet.entry_id);

            Ok(None)
        }
        Packet::DeleteAllEntries(delet) => {
            if delet.magic == 0xD06CB27A {
                let mut state = state.lock().unwrap();
                state.entries_mut().clear()
            }

            Ok(None)
        }
        Packet::EntryUpdate(update) => {
            let mut state = state.lock().unwrap();

            debug!("Got update: {:?}", update);
            state.handle_entry_updated(update);

            Ok(None)
        }
        Packet::EntryFlagsUpdate(update) => {
            let mut state = state.lock().unwrap();

            state.handle_flags_updated(update);

            Ok(None)
        }
        _ => Ok(None)
    }
}

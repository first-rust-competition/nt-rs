
use nt_network::types::EntryValue;
use crate::nt::{EntryData, callback::CallbackType};
use std::collections::HashMap;
use futures_channel::mpsc::Receiver;

pub mod client;
pub mod server;
#[cfg(feature = "websocket")]
pub mod ws;

pub trait NTBackend {
    type State: State;
}

pub struct Client;
impl NTBackend for Client {
    type State = client::ClientState;
}

pub struct Server;
impl NTBackend for Server {
    type State = server::ServerState;
}

pub trait State {
    fn entries(&self) -> &HashMap<u16, EntryData>;

    fn entries_mut(&mut self) -> &mut HashMap<u16, EntryData>;

    fn create_entry(&mut self, data: EntryData) -> Receiver<u16>;

    fn delete_entry(&mut self, id: u16);

    fn update_entry(&mut self, id: u16, new_value: EntryValue);

    fn update_entry_flags(&mut self, id: u16, flags: u8);

    fn clear_entries(&mut self);

    fn add_callback(&mut self, callback_type: CallbackType, action: impl FnMut(&EntryData) + Send + 'static);
}
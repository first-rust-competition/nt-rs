mod client;
mod server;

#[cfg(feature = "websocket")]
mod ws;

pub(crate) use self::client::{ClientCore, ClientState};
pub(crate) use self::server::{ServerCore, ServerState};

use nt_network::types::EntryValue;
use crate::nt::{EntryData, callback::CallbackType};
use crossbeam_channel::Receiver;
use std::collections::HashMap;
use nt_network::{ReceivedPacket, Packet};
use tokio::prelude::*;

pub trait NTBackend {
    type State: State;
}

pub struct Server;
impl NTBackend for Server {
    type State = ServerState;
}

pub struct Client;
impl NTBackend for Client {
    type State = ClientState;
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
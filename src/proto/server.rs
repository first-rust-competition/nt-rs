use crate::proto::State;
use crate::{EntryData, CallbackType, EntryValue};
use std::collections::HashMap;
use futures_channel::mpsc::Receiver;

pub struct ServerState {

}

impl State for ServerState {
    fn entries(&self) -> &HashMap<u16, EntryData> {
        unimplemented!()
    }

    fn entries_mut(&mut self) -> &mut HashMap<u16, EntryData> {
        unimplemented!()
    }

    fn create_entry(&mut self, data: EntryData) -> Receiver<u16> {
        unimplemented!()
    }

    fn delete_entry(&mut self, id: u16) {
        unimplemented!()
    }

    fn update_entry(&mut self, id: u16, new_value: EntryValue) {
        unimplemented!()
    }

    fn update_entry_flags(&mut self, id: u16, flags: u8) {
        unimplemented!()
    }

    fn clear_entries(&mut self) {
        unimplemented!()
    }

    fn add_callback(&mut self, callback_type: CallbackType, action: impl FnMut(&EntryData) + Send) {
        unimplemented!()
    }
}
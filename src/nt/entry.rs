use crate::proto::NTBackend;
use crate::NetworkTables;
use nt_network::types::{EntryType, EntryValue};

#[derive(Clone, Debug, PartialEq)]
pub struct EntryData {
    pub name: String,
    pub flags: u8,
    pub value: EntryValue,
    pub seqnum: u16,
}

impl EntryData {
    pub fn new(name: String, flags: u8, value: EntryValue) -> EntryData {
        Self::new_with_seqnum(name, flags, value, 1)
    }

    pub fn entry_type(&self) -> EntryType {
        self.value.entry_type()
    }

    #[doc(hidden)]
    pub(crate) fn new_with_seqnum(
        name: String,
        flags: u8,
        value: EntryValue,
        seqnum: u16,
    ) -> EntryData {
        EntryData {
            name,
            flags,
            value,
            seqnum,
        }
    }
}

pub struct Entry<'a, T: NTBackend> {
    nt: &'a NetworkTables<T>,
    id: u16,
}

impl<'a, T: NTBackend> Entry<'a, T> {
    pub fn new(nt: &'a NetworkTables<T>, id: u16) -> Entry<'a, T> {
        Entry { nt, id }
    }

    pub fn id(&self) -> &u16 {
        &self.id
    }

    pub fn value(&self) -> EntryData {
        self.nt.entries()[&self.id].clone()
    }

    pub fn set_persistent(&mut self, persistent: bool) {
        self.nt.update_entry_flags(self.id, persistent as u8);
    }

    pub fn set_value(&mut self, new_value: EntryValue) {
        self.nt.update_entry(self.id, new_value);
    }

    pub fn delete(self) {
        self.nt.delete_entry(self.id);
    }
}

use nt_network::types::{EntryValue, EntryType};
use super::NetworkTables;

/// Contains the data associated with an entry
#[derive(Clone, Debug, PartialEq)]
pub struct EntryData {
    pub name: String,
    pub flags: u8,
    pub value: EntryValue,
    pub seqnum: u16,
}

impl EntryData {
    /// Creates a new `EntryData` with the given `name`, `flags`, and `value`, and with a sequence number of 1
    pub fn new(name: String, flags: u8, value: EntryValue) -> EntryData {
        Self::new_with_seqnum(name, flags, value, 1)
    }

    /// Returns the entry type for the stored value
    pub fn entry_type(&self) -> EntryType {
        self.value.entry_type()
    }

    #[doc(hidden)]
    pub(crate) fn new_with_seqnum(name: String, flags: u8, value: EntryValue, seqnum: u16) -> EntryData {
        EntryData {
            name,
            flags,
            value,
            seqnum,
        }
    }
}

/// An entry associated with a NetworkTables connection
pub struct Entry<'a> {
    nt: &'a NetworkTables,
    id: u16,
}

impl<'a> Entry<'a> {
    /// Creates a new `Entry` for the given connection, with the given id
    pub fn new(nt: &'a NetworkTables, id: u16) -> Entry<'a> {
        Entry {
            nt,
            id,
        }
    }

    /// Returns the id of this entry
    pub fn id(&self) -> &u16 {
        &self.id
    }

    /// Returns the data for this entry
    pub fn value(&self) -> EntryData {
        self.nt.entries()[&self.id].clone()
    }

    /// Sets the persistent bit in the entry's flags to `persistent`
    pub fn set_persistent(&mut self, persistent: bool) {
        self.nt.update_entry_flags(self.id, persistent as u8);
    }

    /// Updates the value associated with this entry
    pub fn set_value(&mut self, new_value: EntryValue) {
        self.nt.update_entry(self.id, new_value);
    }

    /// Deletes this entry
    /// `self` is moved into this call, as the entry will be inaccessible after it is deleted
    pub fn delete(self) {
        self.nt.delete_entry(self.id);
    }
}

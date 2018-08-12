use proto::types::{EntryValue, EntryData};
use nt::NetworkTables;

/// Struct representing an immutable entry in NetworkTables with the given `id`
pub struct Entry<'a> {
    nt: &'a NetworkTables,
    id: u16
}

/// Struct representing an entry in NetworkTables with the given id
/// This struct allows for changes to the entry's flags, and value.
pub struct EntryMut<'a> {
    nt: &'a mut NetworkTables,
    id: u16
}

impl<'a> Entry<'a> {
    /// Creates a new [`Entry`] given `nt` and `id`
    pub(crate) fn new(nt: &'a NetworkTables, id: u16) -> Entry<'a> {
        Entry {
            nt,
            id
        }
    }

    /// Returns the id of `self`
    pub fn id(&self) -> &u16 {
        &self.id
    }

    /// Returns the data associated with `self`
    pub fn value(&self) -> EntryData {
        let lock = self.nt.state.lock().unwrap();
        lock.get_entry(self.id).clone()
    }
}

impl<'a> EntryMut<'a> {
    /// Creates a new [`EntryMut`] given `nt` and `id`
    pub(crate) fn new(nt: &'a mut NetworkTables, id: u16) -> EntryMut<'a> {
        EntryMut {
            nt,
            id
        }
    }

    /// Returns the id of `self`
    pub fn id(&self) -> &u16 {
        &self.id
    }

    /// Returns the data associated with `self`
    pub fn value(&self) -> EntryData {
        let lock = self.nt.state.lock().unwrap();
        lock.get_entry(self.id).clone()
    }

    /// Updates the flags of `self`, setting or unsetting the persistence bit
    pub fn set_persistent(&mut self, persistent: bool) {
        self.nt.update_entry_flags(self.id, persistent as u8);
    }

    /// Updates the value of self. new_value must be the same [`EntryType`] as that of `self.value()`
    pub fn set_value(&mut self, new_value: EntryValue) {
        self.nt.update_entry(self.id, new_value);
    }

    /// Converts this into an immutable [`Entry`].
    pub fn into_entry(self) -> Entry<'a> {
        Entry::new(self.nt, self.id)
    }

    /// Deletes the entry associated with `self`
    /// `self` is moved as the data associated with it becomes inaccessible after this operation
    pub fn delete(self) {
        self.nt.delete_entry(self.id)
    }
}
use ::EntryData;

/// Enum representing the types of actions that can notify callbacks when they occur
#[derive(PartialEq, Eq, Hash, Clone)]
pub enum CallbackType {
    /// Notifies callbacks when a new value is added
    Add,
    /// Notifies callbacks when an existing value was validly updated
    Update,
    /// Notifies callbacks when a value was deleted
    Delete
}

#[doc(hidden)]
pub type Action = Fn(&EntryData) + Send + 'static;

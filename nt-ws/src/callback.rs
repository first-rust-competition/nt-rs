use crate::EntryData;
use wasm_bindgen::prelude::*;

/// The types of callbacks that can be registered
#[wasm_bindgen]
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum CallbackType {
    Add,
    Delete,
    Update,
}

pub(crate) type Action = dyn FnMut(&EntryData) + 'static;

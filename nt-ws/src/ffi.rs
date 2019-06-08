//! Contains wrapper types around higher level types that can't be directly translated to WASM
//! This module should not be used directly, but rather through the javascript API

use super::NetworkTables as RNetworkTables;
use super::EntryData as REntryData;
use js_sys::{Uint8Array, Promise, Float64Array, Array, Function, Map};
use futures::prelude::*;
use wasm_bindgen_futures::futures_0_3::future_to_promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use nt_network::types::{EntryType, EntryValue};
use std::iter::{FromIterator, IntoIterator};
use super::CallbackType;

/// The core of nt-js
/// This type represents a connection to a remote NetworkTables server via a websocket
/// It allows for manipulation of entries on that server, including reading, and writing
#[wasm_bindgen]
pub struct NetworkTables {
    inner: RNetworkTables,
}

/// Represents the data associated with an entry
/// Contains the name, flags, type, and value associated with the given entry
#[wasm_bindgen]
pub struct EntryData {
    inner: REntryData,
}

#[wasm_bindgen]
impl EntryData {
    /// Creates a new `EntryData` from the component parts
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, flags: u8, entry_type: EntryType, value: JsValue) -> EntryData {
        let inner_value = js_to_entry_value(entry_type, value);

        EntryData { inner: REntryData::new(name, flags, inner_value) }
    }

    fn from_inner(inner: REntryData) -> EntryData {
        EntryData { inner }
    }

    /// Returns the entry type associated with this value
    pub fn entry_type(&self) -> EntryType { self.inner.value.entry_type() }

    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    pub fn flags(&self) -> u8 {
        self.inner.flags
    }

    pub fn seqnum(&self) -> u16 {
        self.inner.seqnum
    }

    pub fn entry_value(&self) -> JsValue {
        match &self.inner.value {
            EntryValue::String(s) => JsValue::from_str(s),
            EntryValue::Double(f) => JsValue::from_f64(*f),
            EntryValue::Boolean(b) => JsValue::from_bool(*b),
            EntryValue::RawData(d) => {
                let arr = Uint8Array::new_with_length(d.len() as u32);
                let a2 = Array::new();
                for i in 0..d.len() {
                    a2.push(&d[i].into());
                }
                arr.set(&a2, 0);
                arr.into()
            }
            EntryValue::BooleanArray(ba) => {
                let arr = Array::new();
                for i in 0..ba.len() {
                    arr.push(&ba[i].into());
                }

                arr.into()
            }
            EntryValue::StringArray(sa) => {
                let arr = Array::new();
                for i in 0..sa.len() {
                    arr.push(&sa[i].clone().into());
                }

                arr.into()
            }
            EntryValue::DoubleArray(da) => {
                let arr = Float64Array::new_with_length(da.len() as u32);
                let a2 = Array::new();

                for i in 0..da.len() {
                    a2.push(&da[i].into());
                }

                arr.set(&a2, 0);

                arr.into()
            }
        }
    }
}

#[wasm_bindgen]
impl NetworkTables {
    /// Connects to the server at `ip`, with the name `client_name`.
    /// Returns a Promise resolving to a `NetworkTables` instance
    pub fn connect(ip: String, client_name: String) -> Promise {
        let future = RNetworkTables::connect(&ip, &client_name)
            .then(|nt| futures::future::ok(JsValue::from(NetworkTables { inner: nt })));

        future_to_promise(future)
    }

    pub fn entries(&self) -> Map {
        let output = Map::new();

        for (id, data) in self.inner.entries().iter() {
            output.set(&JsValue::from(*id), &JsValue::from(EntryData::from_inner(data.clone())));
        }

        output
    }

    /// Creates a new entry on the server with the given `data`
    /// Returns a promise resolving to the id of the new entry
    pub fn create_entry(&self, data: EntryData) -> Promise {
        let future = self.inner.create_entry(data.inner)
            .then(|id| futures::future::ok(JsValue::from(id)));

        future_to_promise(future)
    }

    /// Deletes the entry with the given id
    pub fn delete_entry(&self, id: u16) {
        self.inner.delete_entry(id);
    }

    /// Deletes all the entries on the server
    pub fn clear_entries(&self) {
        self.inner.clear_entries();
    }

    /// Updates the entry with id `id` to contain the given value
    pub fn update_entry(&self, id: u16, new_value: JsValue) {
        let e = &self.inner.entries()[&id];

        self.inner.update_entry(id, js_to_entry_value(e.entry_type(), new_value));
    }

    /// Updates the entry flags associated with `id`
    pub fn update_entry_flags(&self, id: u16, flags: u8) {
        self.inner.update_entry_flags(id, flags);
    }

    pub fn add_callback(&self, callback_type: CallbackType, callback: Function) {
        let this = JsValue::NULL;
        self.inner.add_callback(callback_type, move |data| {
            let value = JsValue::from(EntryData::from_inner(data.clone()));
            callback.call1(&this, &value).unwrap();
        })
    }
}

/// Translates a raw JsValue into the EntryValue enum, using the given EntryType to infer what it should be cast as
/// TODO: In the future make the inference based on the value itself?
fn js_to_entry_value(entry_type: EntryType, value: JsValue) -> EntryValue {
    match entry_type {
        EntryType::String => EntryValue::String(value.as_string().unwrap()),
        EntryType::Boolean => EntryValue::Boolean(value.as_bool().unwrap()),
        EntryType::Double => EntryValue::Double(value.as_f64().unwrap()),
        EntryType::RawData => {
            let mut v = Vec::new();
            let arr = value.dyn_into::<Uint8Array>().unwrap();
            arr.copy_to(&mut v[..]);

            EntryValue::RawData(v)
        }
        EntryType::BooleanArray => {
            EntryValue::BooleanArray(Vec::<bool>::from_iter(value.dyn_into::<Array>().unwrap().values().into_iter()
                .map(|next| next.unwrap().as_bool().unwrap())))
        }
        EntryType::DoubleArray => {
            let mut v = Vec::new();
            let arr = value.dyn_into::<Float64Array>().unwrap();
            arr.copy_to(&mut v[..]);
            EntryValue::DoubleArray(v)
        }
        EntryType::StringArray => {
            EntryValue::StringArray(Vec::<String>::from_iter(value.dyn_into::<Array>().unwrap().values().into_iter()
                .map(|next| next.unwrap().as_string().unwrap())
            ))
        }
    }
}

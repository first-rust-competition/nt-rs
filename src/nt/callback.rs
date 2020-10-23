use crate::nt::{Topic, TopicSnapshot};
use std::net::SocketAddr;
use std::panic::RefUnwindSafe;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum CallbackType {
    Create,
    Delete,
    Update,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum ConnectionCallbackType {
    ClientConnected,
    ClientDisconnected,
}

pub type ConnectionCallback = dyn FnMut(&SocketAddr, bool) + Send + Sync + 'static;

pub type ValueCallback = dyn FnMut(&TopicSnapshot, CallbackType) + Send + Sync + 'static;


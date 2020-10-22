use crate::nt::Topic;
use std::net::SocketAddr;
use std::panic::RefUnwindSafe;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum CallbackType {
    Add,
    Delete,
    Update,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum ConnectionCallbackType {
    ClientConnected,
    ClientDisconnected,
}

pub type ConnectionCallback = dyn FnMut(&SocketAddr, bool) + Send + Sync + 'static;

pub type Action = dyn FnMut(&Topic) + Send + 'static;


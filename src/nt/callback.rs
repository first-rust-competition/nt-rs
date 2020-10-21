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

pub type ConnectionAction = dyn FnMut(&SocketAddr) + Send + 'static;

pub type Action = dyn FnMut(&Topic) + Send + 'static;

pub type RpcAction = dyn Fn(Vec<u8>) -> Vec<u8> + Send + Sync + RefUnwindSafe + 'static;

pub type RpcCallback = dyn Fn(Vec<u8>) + Send + 'static;

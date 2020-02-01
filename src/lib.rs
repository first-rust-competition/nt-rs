//! # nt
//!
//! `nt` is an library implementing client and server functionality for the NetworkTables rev. 3 protocol
//!
//! The provided [`NetworkTables`](struct.NetworkTables.html) struct contains methods for querying
//! the state of the connection, accessing, as well as updating and creating entries that will be
//! synced to the server.

extern crate tokio;

mod nt;
mod proto;
pub mod error;

/// Base result type for nt-rs
pub type Result<T> = std::result::Result<T, error::Error>;

pub use self::nt::callback::*;
pub use self::nt::entry::EntryData;
pub use self::nt::NetworkTables;
//pub use self::proto::{NTBackend, State, Server, Client};
pub use nt_network::types::*;

//! # nt
//!
//! `nt` is an library implementing client and server functionality for the NetworkTables rev. 3 protocol
//!
//! The provided [`NetworkTables`](struct.NetworkTables.html) struct contains methods for querying
//! the state of the connection, accessing, as well as updating and creating entries that will be
//! synced to the server.

extern crate tokio;

mod backend;
pub mod error;
mod nt;
mod proto;

/// Base result type for nt-rs
pub type Result<T> = std::result::Result<T, error::Error>;

// pub use self::nt::callback::*;
// pub use self::nt::entry::EntryData;
pub use self::backend::{NTBackend, Server, State};
pub use self::nt::NetworkTables;

/// An NT4 time source using system time
/// This is a time source that will work on any machine that wishes to host an NT server,
/// however there are cases when a different time source would be preferred (e.g. FPGA on the roboRIO)
#[cfg(feature = "chrono")]
pub fn system_time() -> u64 {
    use chrono::Local;
    let now = Local::now();
    now.timestamp_nanos() as u64 / 1000
}

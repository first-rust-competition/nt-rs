//! # The protocol for NetworkTables v4 (NT over WebSockets)
//!
//! This module contains Rust representations of the messages described in the NTv4 specification.
//! These types are serialized with the relevant `serde` crates, and can be used to create a client or server compliant with the specification
//!
//! This crate does not prescribe a runtime that must be used, text messages are decoded as normal using serde_json,
//! and binary messages can be decoded from a normal byte slice. Implementation details of the runtime used are left to consumers of the crate.

mod bin;
mod text;

pub mod codec;
mod ext;

pub mod prelude {
    pub use crate::proto::bin::*;
    pub use crate::proto::text::*;

    /// An enum wrapping the possible frames that can be received in NT4 communications
    #[derive(Debug)]
    pub enum NTMessage {
        /// A JSON-encoded message framed as WS TEXT
        Text(Vec<NTTextMessage>),
        /// A Msgpack-encoded data stream framed as WS BIN
        Binary(Vec<NTBinaryMessage>),
        /// A WS CLOSE frame
        Close,
    }

    impl NTMessage {
        pub fn single_text(msg: NTTextMessage) -> NTMessage {
            NTMessage::Text(vec![msg])
        }

        pub fn single_bin(msg: NTBinaryMessage) -> NTMessage {
            NTMessage::Binary(vec![msg])
        }
    }
}

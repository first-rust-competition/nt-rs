//! Strongly typed message bodies for messages related to publishing new entries
//!
//! These messages are sent for the creation and deletion of new entries, and contain the metadata necessary to begin publishing
//! values over CBOR.
use super::{DataType, MessageBody};
use serde::{Deserialize, Serialize};

/// Publish Request Message
///
/// Sent from a client to a server to indicate the client wishes to start publishing values at the given NetworkTables key
///
/// The server will respond with an [`Announce`] message that contains the ID the client can use to publish this value
///
/// When the client no longer wishes to publish to this key, it can send the [`PublishRel`] message to indicate that to the server.
/// This also allows the client to delete the value, if it wishes.
///
/// [`PublishRel`]: ./struct.PublishRel.html
/// [`Announce`]: ../directory/struct.Announce.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PublishReq {
    /// The NetworkTables key
    pub name: String,
    /// The type of the data that the client wishes to start publishing
    #[serde(rename = "type")]
    pub _type: DataType,
}

/// Publish Release Message
///
/// Sent by a client to indicate that it no longer wishes to publish values at the given ID.
/// The client can also request that the associated key can be deleted.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PublishRel {
    pub name: String,
}

/// Set Flags Message
///
/// Sent by a client to indicate that it wishes to update the flags associated with the given entry
///
/// The server will respond with an announce message containing the updated entry
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SetFlags {
    /// The name of the entry to update
    pub name: String,
    /// The flags the client wishes to add
    pub add: Vec<String>,
    /// The flags the client wishes to remove
    pub remove: Vec<String>,
}

impl_message!(PublishReq, SetFlags, PublishRel);

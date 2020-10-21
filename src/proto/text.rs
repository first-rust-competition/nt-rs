use crate::proto::bin::NTValue;
use crate::proto::text::directory::*;
use crate::proto::text::publish::*;
use crate::proto::text::subscription::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

macro_rules! impl_message {
    ($($name:ident),+) => {
        $(
        impl MessageBody for $name {
            fn into_message(self) -> $crate::proto::text::NTTextMessage {
                $crate::proto::text::NTTextMessage {
                    _type: $crate::proto::text::MessageType::$name,
                    data: serde_json::to_value(self).unwrap()
                }
            }
        }
        )+
    }
}

pub mod directory;
pub mod publish;
pub mod subscription;

pub trait MessageBody {
    fn into_message(self) -> NTTextMessage;
}

/// The type of the message that is being sent or received
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    /// Publish Request Message
    /// Direction: Client to Server
    /// Response: Publish Acknowledge
    ///
    /// Sent from a client to the server to indicate the client wants to start publishing values at the given NetworkTables key.
    /// The server will respond with a “puback” message.
    /// Once the client receives the “puback” message it can start publishing data value updates via binary CBOR messages.
    #[serde(rename = "publish")]
    PublishReq,
    /// Publish Release Message
    /// Direction: Client to Server
    ///
    /// Sent from a client to the server to indicate the client wants to stop publishing values at the given NetworkTables key.
    /// The client may also request the key be deleted.
    /// The client **must** stop publishing data value updates via binary CBOR messages prior to sending this message.
    #[serde(rename = "pubrel")]
    PublishRel,
    /// Set Flags Message
    /// Direction: Client to Server
    ///
    /// Sent from a client to the server to set or clear flags for a given topic.
    /// The server will respond with an updated “announce” message.
    SetFlags,
    /// Key Announcement Message
    /// Direction: Server to Client
    ///
    /// Sent from the server to a client with an announcement listener covering the key.
    /// The server shall send this message either initially after receiving Start Announcements from a client,
    /// or when new keys are created with the prefix specified.
    Announce,
    /// Key Removed Message
    /// Direction: Server to Client
    ///
    /// Sent from the server to a client with an announcement listener covering the key.
    /// The server shall send this message when a previously announced (via an “announce” message) key is deleted.
    Unannounce,
    /// Get Values Message
    /// Direction: Client to Server
    /// Response: Values over CBOR
    ///
    /// Sent from a client to the server to indicate the client wants to get the current values for the specified keys (identifiers).
    /// The server shall send CBOR messages containing the current values immediately upon receipt.
    /// While this message could theoretically be used to poll for value updates, it is much better to use the “subscribe” message to request periodic push updates.
    GetValues,
    /// Subscribe Message
    /// Direction: Client to Server
    /// Response: Values over CBOR
    ///
    /// Sent from a client to the server to indicate the client wants to subscribe to value changes for the specified keys (identifiers).
    /// The server shall send CBOR messages containing the current values upon receipt, and continue sending CBOR messages for future value changes.
    /// Subscriptions may overlap; only one CBOR message is sent per value change regardless of the number of subscriptions.
    /// Sending a “subscribe” message with the same subscription UID as a previous “subscribe” message results in updating the subscription (replacing the array of identifiers and updating any specified options).
    Subscribe,
    /// Unsubscribe Message
    /// Direction: Client to Server
    ///
    /// Sent from a client to the server to indicate the client wants to stop subscribing to value changes for the given subscription.
    Unsubscribe,
}

/// An enum containing the structs representing each text message, the explanation of each message can be found in the documentation for [`MessageType`]
///
/// [`MessageType`]: ./enum.MessageType.html
#[derive(Debug, PartialEq)]
pub enum MessageValue {
    PublishReq(PublishReq),
    PublishRel(PublishRel),
    SetFlags(SetFlags),
    Announce(Announce),
    Unannounce(Unannounce),
    GetValues(GetValues),
    Subscribe(Subscribe),
    Unsubscribe(Unsubscribe),
}

/// An enum representation of the acceptable data types in NTv4
#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    /// Represents a boolean, true or false
    Boolean,
    /// Represents a sequence of raw bytes
    Raw,
    /// Represents a Remote Procedure Call declaration
    RPC,
    /// Represents a sequence of bytes representing a String
    String,
    /// Represents a signed 64-bit integer
    Int,
    /// Represents an IEEE754 single-precision floating-point number
    Float,
    /// Represents an IEEE754 double-precision floating-point number
    Double,
    /// Represents an array of Booleans
    #[serde(rename = "boolean[]")]
    BooleanArray,
    /// Represents an array of Strings
    #[serde(rename = "string[]")]
    StringArray,
    /// Represents an array of Integers
    #[serde(rename = "int[]")]
    IntArray,
    /// Represents an array of Floats
    #[serde(rename = "float[]")]
    FloatArray,
    /// Represents an array of Doubles
    #[serde(rename = "double[]")]
    DoubleArray,
}

impl DataType {
    pub fn default_value(&self) -> NTValue {
        match self {
            DataType::Int => NTValue::Int(0),
            DataType::Boolean => NTValue::Boolean(false),
            DataType::Raw => NTValue::Raw(vec![]),
            DataType::RPC => NTValue::RPC(vec![]),
            DataType::String => NTValue::String(String::new()),
            DataType::Float => NTValue::Float(0f32),
            DataType::Double => NTValue::Double(0.0),
            DataType::BooleanArray => NTValue::BooleanArray(vec![]),
            DataType::StringArray => NTValue::StringArray(vec![]),
            DataType::IntArray => NTValue::IntArray(vec![]),
            DataType::FloatArray => NTValue::FloatArray(vec![]),
            DataType::DoubleArray => NTValue::DoubleArray(vec![]),
        }
    }
}

impl Into<u8> for DataType {
    fn into(self) -> u8 {
        match self {
            DataType::Boolean => 0,
            DataType::Double => 1,
            DataType::Int => 2,
            DataType::Float => 3,
            DataType::String => 4,
            DataType::Raw => 5,
            DataType::RPC => 6,
            DataType::BooleanArray => 16,
            DataType::DoubleArray => 17,
            DataType::IntArray => 18,
            DataType::FloatArray => 19,
            DataType::StringArray => 20,
        }
    }
}

/// The most generic struct representing a textual message transmitted in NT4
///
/// This struct should probably not be used directly, and instead can be constructed from the implementors of [`MessageBody`], found in submodules
/// These implementors are strongly typed equivalents to the `data` field on this type, and contain more information about how they should be used.
///
/// [`MessageBody`]: ./trait.MessageBody.html
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct NTTextMessage {
    #[serde(rename = "type")]
    _type: MessageType,
    data: Value,
}

macro_rules! to_data_body {
    ($self:ident, $($ty:ident),+) => {
        match $self._type {
            $(
            MessageType::$ty => match serde_json::from_value::<$ty>($self.data) {
                Ok(value) => Ok(MessageValue::$ty(value)),
                Err(e) => Err(e),
            }
            )+
        }
    }
}

impl NTTextMessage {
    /// Decodes the `Value` stored in `self` as a strongly typed struct depending on the value of `self._type`
    ///
    /// Returns the value wrapped inside the [`MessageValue`] enum.
    ///
    /// [`MessageValue`]: ./enum.MessageValue.html
    pub fn data(self) -> serde_json::Result<MessageValue> {
        use self::directory::*;
        use self::publish::*;
        use self::subscription::*;
        to_data_body!(
            self,
            PublishReq,
            PublishRel,
            SetFlags,
            Announce,
            Unannounce,
            GetValues,
            Subscribe,
            Unsubscribe
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::proto::text::publish::{PublishReq, SetFlags};
    use crate::proto::text::{DataType, MessageBody, MessageType, MessageValue, NTTextMessage};

    #[test]
    fn test_de() {
        let msg = r#"{"type":"publish", "data": {"name": "/foo", "type": "integer"}}"#;
        let msg = serde_json::from_str::<NTTextMessage>(msg).unwrap();
        assert_eq!(msg._type, MessageType::PublishReq);
        assert_eq!(
            msg.data(),
            MessageValue::PublishReq(PublishReq {
                name: "/foo".to_string(),
                _type: DataType::Int,
            })
        );
    }

    #[test]
    fn test_ser() {
        let msg = SetFlags {
            name: "/foo".to_string(),
            add: vec!["persistent".to_string()],
            remove: vec!["bolb".to_string()],
        };

        assert_eq!(
            serde_json::to_string(&msg.into_message()).unwrap(),
            r#"{"type":"setflags","data":{"add":["persistent"],"name":"/foo","remove":["bolb"]}}"#
        )
    }
}

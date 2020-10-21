//! This module handles the binary half of the NTv4 protocol.
//!
//! **Value** updates in NTv4 are sent as WS Binary messages, and are encoded with the Compact Binary Object Representation (CBOR)
//! as defined in RFC 7049.

use crate::proto::ext::*;
use crate::proto::text::DataType;
use rmpv::Value;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use thiserror::Error;

macro_rules! impl_conversion {
    ($self:ident, $($inst:ident),+) => {
        match $self {
            $(
            NTValue::$inst(_) => DataType::$inst,
            )+
        }
    }
}

/// A NetworkTables value
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NTValue {
    /// An integer value. This value stores both signed and unsigned integers in an `i64`
    #[serde(serialize_with = "integer_serializer")]
    Int(i64),
    Float(f32),
    /// A floating-point value. This value represents both single and double precision floats, and is stored in an `f64`
    Double(f64),
    /// A boolean value
    Boolean(bool),
    /// A Raw value, stored as a Vec of the raw bytes
    #[serde(serialize_with = "raw_serializer")]
    Raw(Vec<u8>),
    #[serde(serialize_with = "raw_serializer")]
    RPC(Vec<u8>),
    /// A String value
    String(String),
    /// An Array of booleans
    BooleanArray(Vec<bool>),
    /// An Array of integers
    IntArray(Vec<i64>),
    FloatArray(Vec<f32>),
    /// An Array of floating-point numbers
    DoubleArray(Vec<f64>),
    /// An Array of strings
    StringArray(Vec<String>),
}

pub fn integer_serializer<S: serde::Serializer>(i: &i64, s: S) -> Result<S::Ok, S::Error> {
    let i = *i;
    if i > 0 {
        s.serialize_u64(i as u64)
    } else {
        s.serialize_i64(i)
    }
}

fn raw_serializer<S: serde::Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_bytes(&v[..])
}

impl NTValue {
    pub fn data_type(&self) -> DataType {
        impl_conversion!(
            self,
            Boolean,
            Double,
            Int,
            Float,
            String,
            Raw,
            RPC,
            BooleanArray,
            DoubleArray,
            IntArray,
            FloatArray,
            StringArray
        )
    }
}

// An error encountered when attempting to deserialize NT4 messages from CBOR
#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("Error decoding ID from binary message. Expected an integer. Found: `{0:?}`")]
    InvalidId(Value),
    #[error("Error decoding timestamp from binary message. Expected an integer. Found: `{0:?}`")]
    InvalidTimestamp(Value),
    #[error("Error decoding NT type from binary message. Expected an integer. Found: `{0:?}`")]
    InvalidTypeFieldType(Value),
    #[error("Error decoding NT type from binary message. Invalid NT type `{0}`.")]
    InvalidTypeFieldValue(u8),
    #[error("Error decoding NT value from binary message. Expected `{0:?}`, found an array.")]
    InvalidArrayType(DataType),
    #[error("Error decoding NT value from binary message. Arrays must be of a uniform type. Expected `{0:?}`, found element `{1:?}`.")]
    NonUniformArray(DataType, Value),
    #[error("Error decoding NT value from binary message. Found unexpected MsgPack value `{0:?}`")]
    InvalidMsgPackValue(Value),
    #[error("Error decoding NT value from binary message. Data tagged as type `{0:?}` but decoded as type `{1:?}`")]
    TypeMismatch(DataType, DataType),
    #[error("Error decoding binary message. Codec Error: {0}")]
    MsgPack(#[from] rmpv::decode::Error),
    #[error("Error decoding binary message. Expected top-level array, found `{0:?}`")]
    InvalidTopLevelValue(Value),
    #[error("Error decoding binary message. Invalid length of top-level array: `{0}`.")]
    InvalidTopLevelArrayLength(usize),
}

/// A binary message received in NT4 communications
#[derive(PartialEq, Debug, Clone)]
pub struct NTBinaryMessage {
    /// The ID associated with the given value
    ///
    /// This value is received from the textual half of the protocol, where its relation with a NetworkTables key is specified.
    pub id: i32,
    /// A timestamp associated with this change
    ///
    /// This timestamp is represented in microseconds
    pub timestamp: u64,
    /// The value associated with this change
    pub value: NTValue,
}

impl Serialize for NTBinaryMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(4))?;
        seq.serialize_element(&self.id)?;
        seq.serialize_element(&self.timestamp)?;
        seq.serialize_element::<u8>(&self.value.data_type().into())?;
        seq.serialize_element(&self.value)?;
        seq.end()
    }
}

macro_rules! unpack_value {
    ($messages:expr => $value:expr, $func:ident, $nt_name:ident) => {
        match $value.$func() {
            Some(v) => NTValue::$nt_name(v),
            None => {
                $messages.push(Err(DecodeError::InvalidMsgPackValue($value.clone())));
                continue;
            }
        }
    };
    ($messages:expr => $value:expr, $func:ident) => {
        match $value.$func() {
            Some(v) => v,
            None => {
                $messages.push(Err(DecodeError::InvalidMsgPackValue($value.clone())));
                continue;
            }
        }
    };
}

macro_rules! unpack_array {
    (($messages:expr, $ty:expr, $label:tt) => $value:expr, $conv_func:ident, $nt_name:ident) => {{
        let v = unpack_value!($messages => $value, as_array);
        let mut arr = Vec::with_capacity(v.len());

        for value in v {
            match value.$conv_func() {
                Some(v) => arr.push(v),
                None => {
                    $messages.push(Err(DecodeError::NonUniformArray($ty, value.clone())));
                    continue $label;
                }
            }
        }

        NTValue::$nt_name(arr)
    }}
}

impl NTBinaryMessage {
    pub fn from_slice(slice: &[u8]) -> Vec<Result<Self, DecodeError>> {
        let de = MsgpackStreamIterator::new(slice);

        let mut messages = Vec::new();

        'outer: for value in de {
            match value {
                Ok(Value::Array(values)) => {
                    let id = match &values[0] {
                        Value::Integer(id) => match id.as_i64() {
                            Some(id) => id as i32,
                            None => {
                                messages.push(Err(DecodeError::InvalidId(values[0].clone())));
                                continue;
                            }
                        },
                        val => {
                            messages.push(Err(DecodeError::InvalidId(val.clone())));
                            continue;
                        }
                    };

                    let timestamp = match &values[1] {
                        Value::Integer(ts) => match ts.as_u64() {
                            Some(ts) => ts,
                            None => {
                                messages
                                    .push(Err(DecodeError::InvalidTimestamp(values[1].clone())));
                                continue;
                            }
                        },
                        value => {
                            messages.push(Err(DecodeError::InvalidTimestamp(value.clone())));
                            continue;
                        }
                    };

                    let ty = match &values[2] {
                        Value::Integer(i) => match i.as_u64() {
                            Some(i) => match i {
                                0 => DataType::Boolean,
                                1 => DataType::Double,
                                2 => DataType::Int,
                                3 => DataType::Float,
                                4 => DataType::String,
                                5 => DataType::Raw,
                                6 => DataType::RPC,
                                16 => DataType::BooleanArray,
                                17 => DataType::DoubleArray,
                                18 => DataType::IntArray,
                                19 => DataType::FloatArray,
                                20 => DataType::StringArray,
                                ty => {
                                    messages
                                        .push(Err(DecodeError::InvalidTypeFieldValue(ty as u8)));
                                    continue;
                                }
                            },
                            None => {
                                messages.push(Err(DecodeError::InvalidTypeFieldType(
                                    values[2].clone(),
                                )));
                                continue;
                            }
                        },
                        val => {
                            messages.push(Err(DecodeError::InvalidTypeFieldType(val.clone())));
                            continue;
                        }
                    };

                    let raw_value = &values[3];
                    let value = match ty {
                        DataType::Int => unpack_value!(messages => raw_value, as_integer, Int),
                        DataType::Boolean => unpack_value!(messages => raw_value, as_bool, Boolean),
                        DataType::Raw => unpack_value!(messages => raw_value, as_bytes, Raw),
                        DataType::RPC => unpack_value!(messages => raw_value, as_bytes, RPC),
                        DataType::String => unpack_value!(messages => raw_value, as_text, String),
                        DataType::Float => unpack_value!(messages => raw_value, as_f32, Float),
                        DataType::Double => unpack_value!(messages => raw_value, as_f64, Double),
                        DataType::BooleanArray => {
                            unpack_array!((messages, ty, 'outer) => raw_value, as_bool, BooleanArray)
                        }
                        DataType::StringArray => {
                            unpack_array!((messages, ty, 'outer) => raw_value, as_text, StringArray)
                        }
                        DataType::IntArray => {
                            unpack_array!((messages, ty, 'outer) => raw_value, as_integer, IntArray)
                        }
                        DataType::FloatArray => {
                            unpack_array!((messages, ty, 'outer) => raw_value, as_f32, FloatArray)
                        }
                        DataType::DoubleArray => {
                            unpack_array!((messages, ty, 'outer) => raw_value, as_f64, DoubleArray)
                        }
                    };

                    messages.push(Ok(Self {
                        id,
                        timestamp,
                        value,
                    }));
                }
                Ok(value) => messages.push(Err(DecodeError::InvalidTopLevelValue(value))),
                Err(e) => messages.push(Err(e)),
            }
        }

        messages
    }
}

#[cfg(test)]
mod tests {
    use super::NTBinaryMessage;
    use crate::proto::bin::NTValue;

    #[test]
    fn test_single_message_stream() {
        let data = vec![
            0x94, 0x2a, 0xce, 0x49, 0x96, 0x02, 0xd2, 0x10, 0x93, 0xc3, 0xc2, 0xc3,
        ];

        let messages = NTBinaryMessage::from_slice(&data[..]);

        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages.into_iter().next().unwrap().ok(),
            Some(NTBinaryMessage {
                id: 42,
                timestamp: 1234567890,
                value: NTValue::BooleanArray(vec![true, false, true])
            })
        )
    }

    #[test]
    fn test_multi_message_stream() {
        let data = vec![
            // ITEM 1
            0x94, 0x2a, 0xce, 0x49, 0x96, 0x02, 0xd2, 0x10, 0x93, 0xc3, 0xc2, 0xc3,
            // ITEM 2
            0x94, 0x45, 0xcd, 0x04, 0xd2, 0x04, 0xa5, 0x48, 0x65, 0x6c, 0x6c, 0x6f,
            // ITEM 3
            0x94, 0xcd, 0x01, 0xa4, 0xcd, 0x16, 0x2e, 0x12, 0x94, 0x01, 0x02, 0x03, 0x04,
        ];

        let messages = NTBinaryMessage::from_slice(&data[..]);

        assert_eq!(messages.len(), 3);

        let mut messages = messages.into_iter();

        assert_eq!(
            messages.next().unwrap().ok(),
            Some(NTBinaryMessage {
                id: 42,
                timestamp: 1234567890,
                value: NTValue::BooleanArray(vec![true, false, true])
            })
        );

        assert_eq!(
            messages.next().unwrap().ok(),
            Some(NTBinaryMessage {
                id: 69,
                timestamp: 1234,
                value: NTValue::String("Hello".to_string())
            })
        );

        assert_eq!(
            messages.next().unwrap().ok(),
            Some(NTBinaryMessage {
                id: 420,
                timestamp: 5678,
                value: NTValue::IntArray(vec![1, 2, 3, 4])
            })
        );
    }

    #[test]
    fn test_empty_array() {
        let data = vec![0x94, 0x01, 0xcd, 0x04, 0xd2, 0x14, 0x90];

        let messages = NTBinaryMessage::from_slice(&data[..]);

        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages.into_iter().next().unwrap().ok(),
            Some(NTBinaryMessage {
                id: 1,
                timestamp: 1234,
                value: NTValue::StringArray(vec![])
            })
        )
    }

    #[test]
    fn test_serialize() {
        let msg = NTBinaryMessage {
            id: 1,
            timestamp: 4242,
            value: NTValue::DoubleArray(vec![]),
        };

        let v = rmp_serde::to_vec(&msg).unwrap();

        assert_eq!(&v[..], &[0x94, 0x01, 0xcd, 0x10, 0x92, 0x11, 0x90]);

        let msg = NTBinaryMessage {
            id: 42,
            timestamp: 1234,
            value: NTValue::Double(1.5),
        };

        let v = rmp_serde::to_vec(&msg).unwrap();

        assert_eq!(
            &v[..],
            &[
                0x94, 0x2a, 0xcd, 0x04, 0xd2, 0x01, 0xcb, 0x3f, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00
            ]
        );
    }

    #[test]
    fn test_serialize_raw() {
        let msg = NTBinaryMessage {
            id: 5,
            timestamp: 4242,
            value: NTValue::Raw(vec![0x42, 0x69, 0x2, 0xa]),
        };

        let v = rmp_serde::to_vec(&msg).unwrap();
        assert_eq!(
            &v[..],
            &[0x94, 0x05, 0xcd, 0x10, 0x92, 0x05, 0xc4, 0x04, 0x42, 0x69, 0x2, 0xa],
            "Vector {:x?}",
            v
        );
    }

    #[test]
    fn test_identity() {
        let msg = NTBinaryMessage {
            id: 5,
            timestamp: 12345,
            value: NTValue::Double(4.2),
        };

        let v = rmp_serde::to_vec(&msg).unwrap();
        let mut msg2 = NTBinaryMessage::from_slice(&v[..]).into_iter();

        let res = msg2.next().unwrap();
        println!("{:?}", res);
        assert_eq!(Some(msg), res.ok());
    }

    #[test]
    fn test_invalid_message() {
        let v = vec![0x93, 0x01, 0xa3, 0x66, 0x6f, 0x6f, 0x05];

        let messages = NTBinaryMessage::from_slice(&v[..]);

        assert_eq!(messages.len(), 1);

        assert!(messages[0].is_err());
    }

    #[test]
    fn test_mixed_messages() {
        let v = vec![
            // ITEM 1
            0x94, 0x01, 0xcd, 0x10, 0x92, 0x02, 0x05, // ITEM 2
            0x94, 0x01, 0xcd, 0x10, 0x92, 0x02, // Data tagged as int but with FP value
            0xcb, 0x3f, 0xbf, 0x97, 0x24, 0x74, 0x53, 0x8e, 0xf3,
        ];

        let messages = NTBinaryMessage::from_slice(&v[..]);
        assert_eq!(messages.len(), 2);

        let mut messages = messages.into_iter();

        assert_eq!(
            messages.next().unwrap().ok(),
            Some(NTBinaryMessage {
                id: 1,
                timestamp: 4242,
                value: NTValue::Int(5),
            })
        );

        assert!(messages.next().unwrap().is_err());
    }

    #[test]
    fn test_value_serializer() {
        // Booleans
        assert_eq!(
            &rmp_serde::to_vec(&NTValue::Boolean(false)).unwrap()[..],
            &[0xc2]
        );
        assert_eq!(
            &rmp_serde::to_vec(&NTValue::Boolean(true)).unwrap()[..],
            &[0xc3]
        );

        // Doubles
        assert_eq!(
            &rmp_serde::to_vec(&NTValue::Double(0.25)).unwrap()[..],
            &[0xcb, 0x3f, 0xd0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            &rmp_serde::to_vec(&NTValue::Float(0.25f32)).unwrap()[..],
            &[0xca, 0x3e, 0x80, 0x00, 0x00]
        );

        // Integers
        assert_eq!(&rmp_serde::to_vec(&NTValue::Int(42)).unwrap()[..], &[0x2a]);
        assert_eq!(
            &rmp_serde::to_vec(&NTValue::Int(-42)).unwrap()[..],
            &[0xd0, 0xd6]
        );

        // Byte strings (Raw/RPC)
        assert_eq!(
            &rmp_serde::to_vec(&NTValue::Raw(vec![0xa, 0x42, 0x13, 0x20])).unwrap()[..],
            &[0xc4, 0x04, 0xa, 0x42, 0x13, 0x20]
        );

        // Text strings
        assert_eq!(
            &rmp_serde::to_vec(&NTValue::String("abcd".to_string())).unwrap()[..],
            &[0xa4, 0x61, 0x62, 0x63, 0x64]
        );

        // Arrays
        assert_eq!(
            &rmp_serde::to_vec(&NTValue::IntArray(vec![1, -2, 3, -4])).unwrap()[..],
            &[0x94, 0x01, 0xfe, 0x03, 0xfc]
        );
    }
}

use crate::proto::bin::DecodeError;
use rmpv::decode::read_value;
use rmpv::{Integer, Value};
use std::io::{ErrorKind, Read};

pub trait ValueExt: Sized {
    fn as_integer(&self) -> Option<i64>;
    fn as_f32(&self) -> Option<f32>;
    fn as_bytes(&self) -> Option<Vec<u8>>;
    fn as_text(&self) -> Option<String>;
}

impl ValueExt for Value {
    fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => i.as_i64().or_else(|| i.as_u64().map(|i| i as i64)),
            _ => None,
        }
    }

    fn as_f32(&self) -> Option<f32> {
        match self {
            Value::F32(f) => Some(*f),
            Value::F64(f) => Some(*f as f32),
            Value::Integer(i) => i.as_f64().map(|f| f as f32),
            _ => None,
        }
    }

    fn as_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Value::Binary(v) => Some(v.clone()),
            _ => None,
        }
    }

    fn as_text(&self) -> Option<String> {
        match self {
            Value::String(s) => s.as_str().map(|s| s.to_string()),
            _ => None,
        }
    }
}

pub struct MsgpackStreamIterator<R: Read> {
    rd: R,
}

impl<R: Read> MsgpackStreamIterator<R> {
    pub fn new(rd: R) -> MsgpackStreamIterator<R> {
        MsgpackStreamIterator { rd }
    }
}

impl<R: Read> Iterator for MsgpackStreamIterator<R> {
    type Item = Result<Value, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = read_value(&mut self.rd);

        match result {
            Ok(v) => Some(Ok(v)),
            Err(e) => {
                if e.kind() == ErrorKind::UnexpectedEof {
                    None
                } else {
                    Some(Err(e.into()))
                }
            }
        }
    }
}

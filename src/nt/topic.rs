use crate::backend::NTBackend;
use crate::proto::prelude::publish::SetFlags;
use crate::proto::prelude::{DataType, NTValue};
use crate::NetworkTables;
use serde::de::{Error, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

struct FlagVisitor;

impl<'de> Visitor<'de> for FlagVisitor {
    type Value = Vec<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a sequence of string elements.")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut flags = vec![];
        while let Some(value) = seq.next_element::<String>()? {
            flags.push(value)
        }

        Ok(flags)
    }
}

#[derive(Default, PartialEq, Debug, Clone)]
pub struct TopicFlags {
    pub persistent: bool,
}

impl Serialize for TopicFlags {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        if self.persistent {
            seq.serialize_element("persistent")?;
        }

        seq.end()
    }
}

impl<'de> Deserialize<'de> for TopicFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let flags = deserializer.deserialize_seq(FlagVisitor)?;
        for flag in flags {
            if flag == "persistent" {
                // short circuit, with the current spec the flags should only ever contain 1 element anyways
                return Ok(TopicFlags { persistent: true });
            }
        }

        Ok(TopicFlags { persistent: false })
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct TopicSnapshot {
    pub name: String,
    pub value: NTValue,
    pub timestamp: u64,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub name: String,
    pub value: NTValue,
    pub timestamp: u64,
    pub flags: TopicFlags,
}

impl Topic {
    pub fn new(name: String, _type: DataType) -> Topic {
        Topic {
            name,
            value: _type.default_value(),
            timestamp: 0,
            flags: TopicFlags::default(),
        }
    }

    pub fn set_value(&mut self, value: NTValue, timestamp: u64) {
        self.value = value;
        self.timestamp = timestamp;
    }

    pub fn snapshot(&self) -> TopicSnapshot {
        TopicSnapshot {
            name: self.name.clone(),
            value: self.value.clone(),
            timestamp: self.timestamp,
        }
    }

    pub fn entry_type(&self) -> DataType {
        self.value.data_type()
    }
}

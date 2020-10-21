use crate::proto::prelude::{NTBinaryMessage, NTMessage};
use itertools::Itertools;

pub fn batch_messages(messages: Vec<NTBinaryMessage>, batch_size: usize) -> Vec<NTMessage> {
    messages
        .into_iter()
        .chunks(batch_size)
        .into_iter()
        .map(|batch| NTMessage::Binary(batch.collect()))
        .collect()
}

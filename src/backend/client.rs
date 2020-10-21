use crate::nt::callback::CallbackType;
use crate::nt::topic::{Topic, TopicFlags};
use crate::proto::prelude::NTValue;
use crate::State;
use async_trait::async_trait;
use std::collections::HashMap;

// pub struct NTClient;
//
// #[async_trait]
// impl State for NTClient {
//     fn topics(&self) -> &HashMap<String, Topic> {
//         unimplemented!()
//     }
//
//     fn topics_mut(&mut self) -> &mut HashMap<String, Topic> {
//         unimplemented!()
//     }
//
//     async fn publish(&mut self, topic: Topic) {
//         unimplemented!()
//     }
//
//     async fn release(&mut self, name: String) {
//         unimplemented!()
//     }
//
//     async fn update_topic(&mut self, name: &str, new_value: NTValue) {
//         unimplemented!()
//     }
//
//     async fn update_topic_flags(&mut self, name: &str, flags: TopicFlags) {
//         unimplemented!()
//     }
//
//     fn add_callback(&mut self, callback_type: CallbackType, action: impl FnMut(&Topic) + Send) {
//         unimplemented!()
//     }
// }

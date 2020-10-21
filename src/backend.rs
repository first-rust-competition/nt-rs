use crate::nt::callback::CallbackType;
use crate::nt::topic::{Topic, TopicFlags};
use crate::proto::prelude::NTValue;
use async_trait::async_trait;
use std::collections::HashMap;

pub mod client;
pub mod server;
mod util;

pub trait NTBackend {
    type State: State;
}

// pub struct Client;
// impl NTBackend for Client {
//     type State = client::NTClient;
// }

pub struct Server;
impl NTBackend for Server {
    type State = server::NTServer;
}

#[async_trait]
pub trait State {
    fn topics(&self) -> &HashMap<String, Topic>;

    fn topics_mut(&mut self) -> &mut HashMap<String, Topic>;

    async fn publish(&mut self, topic: Topic);

    async fn release(&mut self, name: &str);

    async fn update_topic(&mut self, name: &str, new_value: NTValue);

    async fn update_topic_flags(&mut self, name: &str, flags: TopicFlags);

    fn add_callback(
        &mut self,
        callback_type: CallbackType,
        action: impl FnMut(&Topic) + Send + 'static,
    );
}

use crate::nt::topic::{Topic, TopicFlags, TopicSnapshot};
use crate::proto::prelude::{
    DataType, MessageBody, NTBinaryMessage, NTMessage, NTTextMessage, NTValue,
};
use crate::State;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::{mpsc, watch, Mutex};

mod client;
use crate::nt::callback::{CallbackType, ConnectionCallbackType, ConnectionCallback, ValueCallback};
use client::ConnectedClient;
use std::sync::Arc;
use crate::backend::server::net::tcp_loop;
use tokio::net::ToSocketAddrs;
use crate::backend::server::loops::{channel_loop, broadcast_loop};
use multimap::MultiMap;
use std::net::SocketAddr;

mod loops;
mod net;

pub const MAX_BATCHING_SIZE: usize = 5;

pub enum ServerMessage {
    ClientDisconnected(u32),
}

struct ServerSubscription {
    prefix: String,
    callback: Box<ValueCallback>
}

pub struct NTServer {
    topics: HashMap<String, Topic>,
    clients: HashMap<u32, ConnectedClient>,
    connection_callbacks: MultiMap<ConnectionCallbackType, Box<ConnectionCallback>>,
    pub_count: HashMap<String, usize>,
    pubs: Vec<String>,
    pub time_source: Box<dyn Fn() -> u64 + Send + Sync + 'static>,
    next_internal_sub_id: u32,
    subscriptions: HashMap<u32, ServerSubscription>,
}

impl NTServer {
    pub async fn new<A: ToSocketAddrs + Send + Sync + 'static>(
        ip: A,
        time_source: impl Fn() -> u64 + Send + Sync + 'static,
        close_rx: watch::Receiver<u8>,
    ) -> Arc<Mutex<NTServer>> {
        //TODO: Secure socket (requires tokio-rustls feature in tungstenite)
        let _self = Arc::new(Mutex::new(NTServer {
            topics: HashMap::new(),
            clients: HashMap::new(),
            connection_callbacks: MultiMap::new(),
            pub_count: HashMap::new(),
            pubs: Vec::new(),
            time_source: Box::new(time_source),
            next_internal_sub_id: 1,
            subscriptions: HashMap::new()
        }));

        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(tcp_loop(_self.clone(), tx, ip, close_rx));
        tokio::spawn(channel_loop(Arc::downgrade(&_self), rx));
        tokio::spawn(broadcast_loop(Arc::downgrade(&_self)));
        _self
    }

    async fn create_topic(&mut self, name: String, _type: DataType) {
        if self.topics.contains_key(&name) {
            let topic = &self.topics[&name];
            if topic.entry_type() == _type {
                let cnt = self.pub_count.get_mut(&name).unwrap();
                *cnt += 1;
                return;
            }
        }
        let topic = Topic::new(name.clone(), _type);

        for sub in self.subscriptions.values_mut().filter(|sub| topic.name.starts_with(&sub.prefix)) {
            (sub.callback)(&topic.snapshot(), CallbackType::Create);
        }

        self.broadcast_announce(&topic).await;

        self.pub_count.insert(name.clone(), 1);
        self.topics.insert(name, topic);
    }

    async fn broadcast_announce(&mut self, topic: &Topic) {
        for (_, client) in self.clients.iter_mut() {
            let msg = client.announce(topic).into_message();
            client.send_message(NTMessage::single_text(msg)).await;
        }
    }

    async fn on_release(&mut self, name: &str) {
        let publishers = self.pub_count.get_mut(name).unwrap();
        *publishers -= 1;

        if *publishers == 0 {
            let topic = self.topics.remove(name).unwrap();

            for client in self.clients.values_mut() {
                let msg = client.unannounce(&topic).into_message();
                client.send_message(NTMessage::single_text(msg)).await;
            }

            for sub in self.subscriptions.values_mut().filter(|sub| topic.name.starts_with(&sub.prefix)) {
                (sub.callback)(&topic.snapshot(), CallbackType::Delete);
            }

            self.pub_count.remove(name);
        }
    }

    fn now(&self) -> u64 {
        (self.time_source)()
    }

    fn add_callback(&mut self, ty: ConnectionCallbackType, cb: impl FnMut(&SocketAddr, bool) + Send + Sync + 'static) {
        self.connection_callbacks.insert(ty, Box::new(cb));
    }

    fn update_value(
        &mut self,
        name: &str,
        value: NTValue,
        timestamp: u64,
    ) -> Option<TopicSnapshot> {
        let topic = self.topics.get_mut(name)?;
        if topic.timestamp <= timestamp && value.data_type() == topic.entry_type() {
            topic.set_value(value, timestamp);
            Some(topic.snapshot())
        } else {
            None
        }
    }
}

#[async_trait]
impl State for NTServer {
    fn topics(&self) -> &HashMap<String, Topic> {
        &self.topics
    }

    fn topics_mut(&mut self) -> &mut HashMap<String, Topic> {
        &mut self.topics
    }

    async fn publish(&mut self, topic: Topic) {
        self.pubs.push(topic.name.clone());
        let ty = topic.entry_type();
        self.create_topic(topic.name, ty).await;
    }

    async fn release(&mut self, name: &str) {
        // Vec::contains, unlike any other container, doesn't let me search with &str instead of &String
        if self.pubs.iter().any(|s| s == name) {
            let (idx, _) = self
                .pubs
                .iter()
                .enumerate()
                .find(|(_, needle)| **needle == *name)
                .unwrap();
            self.pubs.remove(idx);

            self.on_release(name).await;
        }
    }

    async fn subscribe(&mut self, prefix: String, callback: Box<ValueCallback>) -> u32 {
        let id = self.next_internal_sub_id;
        self.next_internal_sub_id.wrapping_add(1);
        let sub = ServerSubscription { prefix, callback };
        self.subscriptions.insert(id, sub);
        id
    }

    async fn unsubscribe(&mut self, subuid: u32) {
        self.subscriptions.remove(&subuid);
    }

    async fn update_topic(&mut self, name: &str, new_value: NTValue) {
        if let Some(topic) = self.update_value(name, new_value, self.now()) {
            for client in self
                .clients
                .values_mut()
                .filter(|client| client.subscribed_to(&topic.name))
            {
                client
                    .send_message(NTMessage::single_bin(NTBinaryMessage {
                        // Some above means that the name is connected to a topic, and since the client is subscribed
                        // it will have an id mapped
                        id: client.lookup_id(&topic.name).unwrap(),
                        timestamp: topic.timestamp,
                        value: topic.value.clone(),
                    }))
                    .await;
            }
        }
    }

    // ow
    async fn update_topic_flags(mut self: &mut Self, name: &str, flags: TopicFlags) {
        let &mut NTServer {
            topics, clients, ..
        } = &mut self;
        if let Some(topic) = topics.get_mut(name) {
            topic.flags = flags;

            for (_, client) in clients.iter_mut() {
                let msg = client.announce(topic).into_message();
                client.send_message(NTMessage::single_text(msg)).await;
            }
        }
    }
}

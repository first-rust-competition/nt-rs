use crate::backend::server::{NTServer, ServerMessage, MAX_BATCHING_SIZE};
use crate::backend::util::batch_messages;
use crate::error::Result;
use crate::nt::topic::{Topic, TopicSnapshot};
use crate::proto::codec::NTSocket;
use crate::proto::prelude::directory::{Announce, Unannounce};
use crate::proto::prelude::subscription::{Subscribe, Unsubscribe};
use crate::proto::prelude::{MessageBody, MessageValue, NTBinaryMessage, NTMessage, NTValue};
use crate::State;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::{Arc, Weak};
use tokio::sync::{mpsc, oneshot, Mutex};
use std::net::SocketAddr;
use crate::nt::callback::ConnectionCallbackType;

#[derive(Clone, Debug)]
pub struct Subscription {
    pub prefixes: Vec<String>,
    pub immediate: bool,
    pub periodic: f64,
    pub logging: bool,
}

pub struct ConnectedClient {
    pub net_tx: SplitSink<NTSocket, NTMessage>,
    pub subs: HashMap<u32, Subscription>,
    pub sub_loop_channels: HashMap<u32, oneshot::Sender<()>>,
    pub pub_ids: HashMap<String, i32>,
    pub pubs: Vec<String>,
    pub queued_updates: Vec<TopicSnapshot>,
    next_pub_id: i32,
}

async fn client_loop(
    mut rx: SplitStream<NTSocket>,
    mut tx: mpsc::Sender<ServerMessage>,
    cid: u32,
    addr: SocketAddr,
    state: Weak<Mutex<NTServer>>,
) -> Result<()> {
    // Simple loop over the incoming packet stream. If the state pointer ever fails to upgrade, then the NetworkTables
    // struct has been dropped so the loop should just bail.
    while let Some(packet) = rx.next().await {
        match packet {
            Ok(msg) => match msg {
                NTMessage::Text(msgs) => {
                    match state.upgrade() {
                        Some(state) => {
                            let mut state = state.lock().await;
                            let state = state.deref_mut();
                            for msg in msgs {
                                match msg.data() {
                                    Ok(msg) => match msg {
                                        MessageValue::PublishReq(req) => {
                                            log::info!("Received publish request");
                                            state.create_topic(req.name.clone(), req._type).await;
                                            let topic = &state.topics[&req.name];

                                            let client = state.clients.get_mut(&cid).unwrap();
                                            client.pubs.push(req.name);
                                        }
                                        MessageValue::PublishRel(rel) => {
                                            let client = state.clients.get_mut(&cid).unwrap();
                                            if let Some((idx, _)) = client
                                                .pubs
                                                .iter()
                                                .enumerate()
                                                .find(|(_, p)| **p == rel.name)
                                            {
                                                client.pubs.remove(idx);
                                            }

                                            state.on_release(&rel.name).await;
                                        }
                                        MessageValue::SetFlags(set) => {
                                            let topic = state.topics.get_mut(&set.name).unwrap();
                                            if !topic.flags.persistent
                                                && set.add.iter().any(|x| x == "persistent")
                                            {
                                                topic.flags.persistent = true;
                                            }
                                            if topic.flags.persistent
                                                && set.remove.iter().any(|x| x == "persistent")
                                            {
                                                topic.flags.persistent = false;
                                            }

                                            for (_, client) in state.clients.iter_mut() {
                                                let msg = client.announce(topic).into_message();
                                                client.send_message(NTMessage::single_text(msg)).await;
                                            }
                                        }
                                        MessageValue::GetValues(gv) => {
                                            let client = state.clients.get_mut(&cid).unwrap();

                                            let mut updates = Vec::new();
                                            for id in gv.ids {
                                                let name = client.id_to_name(id).unwrap();
                                                let topic = &state.topics[name];
                                                updates.push(NTBinaryMessage {
                                                    id,
                                                    timestamp: topic.timestamp,
                                                    value: topic.value.clone(),
                                                });
                                            }

                                            for msg in batch_messages(updates, MAX_BATCHING_SIZE) {
                                                client.send_message(msg).await;
                                            }
                                        }
                                        MessageValue::Subscribe(sub) => {
                                            let client = state.clients.get_mut(&cid).unwrap();

                                            let mut updates = Vec::new();
                                            for prefix in &sub.prefixes {
                                                for (_, topic) in state
                                                    .topics
                                                    .iter()
                                                    .filter(|(key, _)| key.starts_with(prefix))
                                                {
                                                    updates.push(NTBinaryMessage {
                                                        id: client.lookup_id(&topic.name).unwrap(),
                                                        timestamp: topic.timestamp,
                                                        value: topic.value.clone(),
                                                    });
                                                }
                                            }

                                            for msg in batch_messages(updates, MAX_BATCHING_SIZE) {
                                                client.send_message(msg).await;
                                            }

                                            let subuid = sub.subuid;
                                            let sub = client.subscribe(sub);
                                            log::info!(
                                                "Subscription registered, perid is {}",
                                                sub.periodic
                                            );
                                            if sub.periodic != 0.1 {
                                                // TODO: Nonstandard broadcast period loop
                                            }
                                        }
                                        MessageValue::Unsubscribe(unsub) => {
                                            let client = state.clients.get_mut(&cid).unwrap();
                                            client.unsubscribe(unsub).await;
                                        }
                                        _ => {}
                                    },
                                    Err(e) => log::error!("Error decoding text message from client: {}", e),
                                }
                            }
                        }
                        None => break,
                    }
                }
                NTMessage::Binary(msgs) => {
                    match state.upgrade() {
                        Some(state) => {
                            let mut state = state.lock().await;
                            let state = state.deref_mut();

                            let client = state.clients.get_mut(&cid).unwrap();

                            let mut updates = Vec::new();
                            for mut msg in msgs {
                                if msg.id == -1 {
                                    let now = (state.time_source)();

                                    // Update the message timestamp and send it back
                                    msg.timestamp = now;
                                    client.send_message(NTMessage::single_bin(msg)).await;
                                    log::info!("Immediately sending timestamp to client");
                                    continue;
                                }

                                log::info!("Performing lookup for ID {}", msg.id);
                                let name = client.id_to_name(msg.id).unwrap();
                                log::info!(
                                    "Received update message for name {}. New value {:?}",
                                    name,
                                    msg.value
                                );

                                let topic = state.topics.get_mut(name).unwrap();
                                if topic.timestamp <= msg.timestamp {
                                    topic.set_value(msg.value, msg.timestamp);
                                    updates.push(topic.snapshot());
                                }
                            }

                            for client in state.clients.values_mut() {
                                let iter = updates
                                    .iter()
                                    .cloned()
                                    .filter(|snapshot| client.subscribed_to(&snapshot.name))
                                    .collect::<Vec<TopicSnapshot>>();

                                for topic in iter {
                                    let sub = client.subscription(&topic.name).unwrap();
                                    if sub.immediate {
                                        client
                                            .send_message(NTMessage::single_bin(NTBinaryMessage {
                                                id: client.lookup_id(&topic.name).unwrap(),
                                                timestamp: topic.timestamp,
                                                value: topic.value.clone(),
                                            }))
                                            .await;
                                    } else {
                                        client.queued_updates.push(topic);
                                    }
                                }
                            }
                        }
                        None => break,
                    }
                }
                NTMessage::Close => {
                    let _ = tx.send(ServerMessage::ClientDisconnected(cid)).await;
                    return Ok(());
                }
            },
            Err(e) => log::error!("Encountered error decoding frame from client: {}", e),
        }
    }

    log::info!("Client loop for CID {} terminated", cid);
    tx.send(ServerMessage::ClientDisconnected(cid)).await;

    // Notify callbacks if they still exist
    if let Some(state) = state.upgrade() {
        let mut state = state.lock().await;
        state.connection_callbacks.iter_all_mut()
            .filter(|(ty, _)| **ty == ConnectionCallbackType::ClientDisconnected)
            .flat_map(|(_, fns)| fns)
            .for_each(|cb| cb(&addr, false))
    }


    Ok(())
}

impl ConnectedClient {
    pub fn new(
        sock: NTSocket,
        server_tx: mpsc::Sender<ServerMessage>,
        cid: u32,
        state: &Arc<Mutex<NTServer>>,
        addr: SocketAddr,
    ) -> ConnectedClient {
        let (net_tx, net_rx) = sock.split();

        tokio::spawn(client_loop(net_rx, server_tx, cid, addr, Arc::downgrade(state)));

        ConnectedClient {
            net_tx,
            subs: HashMap::new(),
            sub_loop_channels: HashMap::new(),
            pub_ids: HashMap::new(),
            queued_updates: Vec::new(),
            next_pub_id: 1,
            pubs: Vec::new(),
        }
    }

    pub fn subscribed_to(&self, name: &str) -> bool {
        self.subs
            .values()
            .any(|sub| sub.prefixes.iter().any(|prefix| name.starts_with(prefix)))
    }

    pub fn subscription(&self, name: &str) -> Option<&Subscription> {
        self.subs
            .values()
            .find(|sub| sub.prefixes.iter().any(|prefix| name.starts_with(prefix)))
    }

    pub fn announce(&mut self, entry: &Topic) -> Announce {
        if let Some(id) = self.pub_ids.get(&entry.name) {
            // Function is called again when updating flags, don't create a new id if that's the case
            Announce {
                name: entry.name.clone(),
                id: *id,
                _type: entry.entry_type(),
                flags: entry.flags.clone(),
            }
        } else {
            let id = self.next_pub_id;
            self.next_pub_id += 1;
            self.pub_ids.insert(entry.name.clone(), id);
            Announce {
                name: entry.name.clone(),
                id,
                _type: entry.entry_type(),
                flags: entry.flags.clone(),
            }
        }
    }

    pub fn unannounce(&mut self, entry: &Topic) -> Unannounce {
        let id = self.pub_ids.remove(&entry.name).unwrap();
        Unannounce {
            name: entry.name.clone(),
            id,
        }
    }

    pub fn id_to_name(&self, id: i32) -> Option<&String> {
        self.pub_ids
            .iter()
            .find(|(_, pub_id)| **pub_id == id)
            .map(|(name, _)| name)
    }

    pub fn lookup_id(&self, name: &str) -> Option<i32> {
        self.pub_ids.get(name).map(|id| *id)
    }

    pub fn subscribe(&mut self, packet: Subscribe) -> Subscription {
        let mut sub = Subscription {
            prefixes: packet.prefixes,
            immediate: false,
            periodic: 0.1,
            logging: false,
        };

        if let Some(opts) = packet.options {
            sub.immediate = opts.immediate;
            sub.logging = opts.logging;
            sub.periodic = opts.periodic;
        }

        self.subs.insert(packet.subuid, sub.clone());
        sub
    }

    pub fn subscribe_channel(&mut self, subuid: u32, ch: oneshot::Sender<()>) {
        self.sub_loop_channels.insert(subuid, ch);
    }

    pub async fn unsubscribe(&mut self, packet: Unsubscribe) {
        self.subs.remove(&packet.subuid);
        if let Some(ch) = self.sub_loop_channels.remove(&packet.subuid) {
            let _ = ch.send(());
        }
    }

    pub async fn send_message(&mut self, msg: NTMessage) {
        let _ = self.net_tx.send(msg).await;
    }
}

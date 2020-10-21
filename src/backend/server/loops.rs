use crate::backend::server::{NTServer, ServerMessage, MAX_BATCHING_SIZE};
use std::sync::{Arc, Weak};
use tokio::sync::Mutex;
use anyhow::Result;
use tokio::sync::{mpsc, oneshot};
use tokio::stream::StreamExt;
use tokio::time::Duration;
use std::ops::DerefMut;
use itertools::Itertools;
use crate::proto::prelude::NTBinaryMessage;
use crate::backend::util::batch_messages;
use crate::backend::server::client::Subscription;
use futures::future::{select, Either};

pub async fn channel_loop(state: Weak<Mutex<NTServer>>, mut rx: mpsc::Receiver<ServerMessage>) -> Result<()> {
    while let Some(msg) = rx.next().await {
        match msg {
            ServerMessage::ClientDisconnected(cid) => {
                log::info!("Received disconnect from CID {}", cid);
                match state.upgrade() {
                    Some(state) => {
                        let mut state = state.lock().await;

                        if let Some(client) = state.clients.remove(&cid) {
                            for key in client.pubs.into_iter() {
                                state.on_release(&key).await;
                            }
                        }
                    }
                    None => return Ok(())
                }
            }
        }
    }
    Ok(())
}

pub async fn broadcast_with_period(
    state: Arc<Mutex<NTServer>>,
    cid: u32,
    sub: Subscription,
    mut rx: oneshot::Receiver<()>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs_f64(sub.periodic));

    loop {
        match select(interval.next(), rx).await {
            Either::Left((Some(_), end_cnt)) => {
                rx = end_cnt;
                let mut state = state.lock().await;
                let state = state.deref_mut();

                let client = state.clients.get_mut(&cid).unwrap();

                let mut updates = Vec::new();
                let mut names = Vec::new();
                for (name, group) in &client
                    .queued_updates
                    .iter()
                    .filter(|snapshot| {
                        sub.prefixes
                            .iter()
                            .any(|prefix| snapshot.name.starts_with(prefix))
                    })
                    .group_by(|snapshot| &snapshot.name)
                {
                    if sub.logging {
                        // Subscribers with this option need to receive every
                        for update in group {
                            updates.push(NTBinaryMessage {
                                id: client.lookup_id(name).unwrap(),
                                timestamp: update.timestamp,
                                value: update.value.clone(),
                            });
                        }
                    } else {
                        let snapshot = group
                            .max_by(|s1, s2| s1.timestamp.cmp(&s2.timestamp))
                            .unwrap();
                        updates.push(NTBinaryMessage {
                            id: client.lookup_id(name).unwrap(),
                            timestamp: snapshot.timestamp,
                            value: snapshot.value.clone(),
                        })
                    }

                    names.push(name.clone());
                }

                for msg in batch_messages(updates, MAX_BATCHING_SIZE) {
                    client.send_message(msg).await;
                }

                for name in names {
                    while let Some((idx, _)) = client
                        .queued_updates
                        .iter()
                        .enumerate()
                        .find(|(_, topic)| topic.name == name)
                    {
                        client.queued_updates.remove(idx);
                    }
                }
            }
            _ => break,
        }
    }

    log::info!(
        "Terminating custom loop for CID {}. (Had period {})",
        cid,
        sub.periodic
    );
}


pub async fn broadcast_loop(state: Weak<Mutex<NTServer>>) {
    let mut interval = tokio::time::interval(Duration::from_millis(100));

    while let Some(_) = interval.next().await {
        match state.upgrade() {
            Some(state) => {
                let mut state = state.lock().await;
                let state = state.deref_mut();

                for client in state.clients.values_mut() {
                    let mut updates = Vec::new();
                    for (_, sub) in &client.subs {
                        if sub.periodic != 0.1 {
                            continue;
                        }
                        if sub.logging {
                            // Subscribers with this option need to receive every
                            for (name, group) in &client
                                .queued_updates
                                .iter()
                                .group_by(|snapshot| &snapshot.name)
                            {
                                for update in group {
                                    updates.push(NTBinaryMessage {
                                        id: client.lookup_id(name).unwrap(),
                                        timestamp: update.timestamp,
                                        value: update.value.clone(),
                                    });
                                }
                            }
                        } else {
                            for (name, group) in &client
                                .queued_updates
                                .iter()
                                .group_by(|snapshot| &snapshot.name)
                            {
                                let snapshot = group
                                    .max_by(|s1, s2| s1.timestamp.cmp(&s2.timestamp))
                                    .unwrap();
                                updates.push(NTBinaryMessage {
                                    id: client.lookup_id(name).unwrap(),
                                    timestamp: snapshot.timestamp,
                                    value: snapshot.value.clone(),
                                })
                            }
                        }

                        client.queued_updates.clear();
                    }

                    for msg in batch_messages(updates, MAX_BATCHING_SIZE) {
                        client.send_message(msg).await;
                    }
                }
            }
            None => return,
        }
    }
}


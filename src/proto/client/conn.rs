use failure::{Error, err_msg};
use futures::sync::mpsc::{Sender, Receiver, UnboundedReceiver, UnboundedSender};
use futures::future::{ok, Either, err};
use tokio::prelude::*;
use tokio::timer::Interval;
use nt_network::{Packet, ReceivedPacket, ClientHelloComplete, KeepAlive};
use crate::proto::ClientState;
use crate::nt::EntryData;
use crate::nt::callback::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn send_packets(tx: impl Sink<SinkItem=Box<dyn Packet>, SinkError=Error>, rx: UnboundedReceiver<Box<dyn Packet>>,
                    handshake_rx: Receiver<()>, packet_tx: UnboundedSender<Box<dyn Packet>>) -> impl Future<Item=(), Error=()> {
    handshake_rx.take(1).map_err(drop)
        .and_then(move |_| tokio::spawn(keepalive_looper(packet_tx.clone())))
        .fold(tx, |tx, _| tx.send(Box::new(ClientHelloComplete)).map_err(drop))
        .and_then(|tx| rx.fold(tx, |tx, packet| {
            tx.send(packet).map_err(drop)
        }))
        .then(|_| Ok(()))
}

pub fn keepalive_looper(tx: UnboundedSender<Box<dyn Packet>>) -> impl Future<Item=(), Error=()> {
    Interval::new_interval(Duration::from_millis(1000))
        .map_err(drop)
        .for_each(move |_| tx.unbounded_send(Box::new(KeepAlive)).map_err(drop))
}

pub fn poll_socket(rx: impl Stream<Item=ReceivedPacket, Error=()>,
                   handshake_tx: Sender<()>,
                   state: Arc<Mutex<ClientState>>,
                   close_rx: Receiver<()>,
                   wait_tx: crossbeam_channel::Sender<()>) -> impl Future<Item=(), Error=()> {
    rx
        .map_err(|_| err_msg("Error from socket stream"))
        .map(Either::A)
        .select(close_rx.map(Either::B).map_err(|_| err_msg("Error from close channel")))
        .fold(handshake_tx, move |tx, packet| {
            match packet {
                Either::A(packet) => {
                    match packet {
                        ReceivedPacket::EntryAssignment(ea) => {
                            let data = EntryData::new(
                                ea.entry_name,
                                ea.entry_flags,
                                ea.entry_value,
                            );
                            let mut state = state.lock().unwrap();
                            if state.pending_entries.contains_key(&data.name) {
                                let tx = state.pending_entries.remove(&data.name).unwrap();
                                tx.send(ea.entry_id).unwrap();
                            }
                            state.callbacks.iter_all_mut()
                                .filter(|&(key, _)| *key == CallbackType::Add)
                                .flat_map(|(_, cbs)| cbs)
                                .for_each(|cb| cb(&data));

                            state.entries.insert(ea.entry_id, data);
                            Either::B(ok(tx))
                        }
                        ReceivedPacket::EntryDelete(ed) => {
                            let mut state = state.lock().unwrap();
                            if state.entries.contains_key(&ed.entry_id) {
                                let data = state.entries.remove(&ed.entry_id).unwrap();
                                state.callbacks.iter_all_mut()
                                    .filter(|&(key, _)| *key == CallbackType::Delete)
                                    .flat_map(|(_, cbs)| cbs)
                                    .for_each(|cb| cb(&data));
                            }
                            Either::B(ok(tx))
                        }
                        ReceivedPacket::ClearAllEntries(cae) => {
                            if cae.is_valid() {
                                state.lock().unwrap().entries.clear();
                            }
                            Either::B(ok(tx))
                        }
                        ReceivedPacket::EntryUpdate(eu) => {
                            let mut state = state.lock().unwrap();
                            if state.entries.contains_key(&eu.entry_id) {
                                {
                                    let mut data = state.entries.get_mut(&eu.entry_id).unwrap();
                                    if eu.entry_seqnum > data.seqnum && eu.entry_type == data.entry_type() {
                                        data.value = eu.entry_value;
                                        data.seqnum = eu.entry_seqnum;
                                    }
                                }

                                let data = state.entries.get(&eu.entry_id).unwrap().clone();
                                state.callbacks.iter_all_mut()
                                    .filter(|&(key, _)| *key == CallbackType::Update)
                                    .flat_map(|(_, cbs)| cbs)
                                    .for_each(|cb| cb(&data))
                            }
                            Either::B(ok(tx))
                        }
                        ReceivedPacket::EntryFlagsUpdate(efu) => {
                            let mut state = state.lock().unwrap();
                            if state.entries.contains_key(&efu.entry_id) {
                                let mut data = state.entries.get_mut(&efu.entry_id).unwrap();
                                data.flags = efu.entry_flags;
                            }
                            Either::B(ok(tx))
                        }
                        ReceivedPacket::ServerHelloComplete => {
                            wait_tx.send(()).unwrap();
                            Either::A(tx.send(()).map_err(|e| e.into()))
                        },
                        _ => Either::B(ok(tx))
                    }
                }
                Either::B(_) => Either::B(err(err_msg("NetworkTables dropped or disconnected")))
            }
        })
        .then(|_| Ok(()))
}


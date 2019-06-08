use tokio::codec::Framed;
use tokio::net::TcpStream;
use tokio::timer::Timeout;
use tokio::prelude::*;
use nt_network::codec::NTCodec;
use std::sync::{Mutex, Arc};
use super::ServerState;
use futures::sync::mpsc::{unbounded, UnboundedSender};
use crate::nt::EntryData;
use crate::nt::callback::*;
use nt_network::*;
use std::net::SocketAddr;
use std::time::Duration;
use futures::future::{err, ok};
use failure::err_msg;
use crate::proto::State;

enum ConnectionState {
    ClientHandshake,
    ServerHandshake,
    Connected,
}

pub fn handle_client(addr: SocketAddr, tx: impl Sink<SinkItem=Box<dyn Packet>, SinkError=failure::Error> + Send + 'static, rx: impl Stream<Item=ReceivedPacket, Error=failure::Error> + Send + 'static, state: Arc<Mutex<ServerState>>) {
    let _state = Arc::new(Mutex::new(ConnectionState::ClientHandshake));

    let (packet_tx, packet_rx) = unbounded::<Box<dyn Packet>>();

    tokio::spawn(packet_rx.fold(tx, move |tx, packet| {
        tx.send(packet).map_err(drop)
    }).map(drop).map_err(drop));

    state.lock().unwrap().add_client(addr, packet_tx.clone());

    let err_state = state.clone();

    tokio::spawn(rx
        .timeout(Duration::new(5, 0)) // Kill the socket after 5 seconds of inactivity. No real metric for if a client is connected but this is spec compliant
        .map_err(|e| {
            if e.is_inner() {
                e.into_inner().unwrap()
            }else {
                err_msg("deadline elapsed")
            }
        })
        .for_each(move |packet| {
            let mut _state = _state.lock().unwrap();
            match *_state {
                ConnectionState::ClientHandshake => {
                    if let ReceivedPacket::ClientHello(hello) = packet {
                        if hello.version != NTVersion::V3 {
                            packet_tx.unbounded_send(Box::new(ProtocolVersionUnsupported { supported_version: NTVersion::V3 as u16 })).unwrap();
                            return Err(err_msg("Invalid Client Version"));
                        }

                        packet_tx.unbounded_send(Box::new(ServerHello::new(0, state.lock().unwrap().server_name.clone()))).unwrap();
                        let state = state.lock().unwrap();
                        for (id, data) in state.entries().iter() {
                            let packet = EntryAssignment::new(
                                data.name.clone(),
                                data.entry_type(),
                                *id,
                                data.seqnum,
                                data.flags,
                                data.value.clone(),
                            );

                            packet_tx.unbounded_send(Box::new(packet)).unwrap();
                        }
                        packet_tx.unbounded_send(Box::new(ServerHelloComplete)).unwrap();

                        *_state = ConnectionState::ServerHandshake;
                        Ok(())
                    } else {
                        Err(err_msg("Unexpected packet for ClientHandshake state"))
                    }
                }
                ConnectionState::ServerHandshake => {
                    match packet {
                        ReceivedPacket::EntryAssignment(ea) => {
                            if ea.entry_id != 0xFFFF {
                                return Err(err_msg("Client sent EntryAssignment with id != 0xFFFF"));
                            }
                            let data = EntryData::new(ea.entry_name, ea.entry_flags, ea.entry_value);
                            let mut state = state.lock().unwrap();
                            let id = state.create_entry(data).recv().unwrap();
                            let data = state.entries().get(&id).unwrap().clone();
                            state.callbacks.iter_all_mut()
                                .filter(|(ty, _)| **ty == CallbackType::Add)
                                .flat_map(|(_, cbs)| cbs)
                                .for_each(|cb| cb(&data))
                        }
                        ReceivedPacket::ClientHelloComplete => {
                            *_state = ConnectionState::Connected;
                            state.lock().unwrap()
                                .server_callbacks.iter_all_mut()
                                .filter(|(ty, _)| **ty == ServerCallbackType::ClientConnected)
                                .flat_map(|(_, cbs)| cbs)
                                .for_each(|cb| cb(&addr));
                        }
                        _ => return Err(err_msg("Invalid packet for given state"))
                    }
                    Ok(())
                }
                ConnectionState::Connected => {
                    match packet {
                        ReceivedPacket::EntryAssignment(ea) => {
                            if ea.entry_id != 0xFFFF {
                                return Err(err_msg("Client sent EntryAssignment with id != 0xFFFF"));
                            }
                            let data = EntryData::new(ea.entry_name, ea.entry_flags, ea.entry_value);
                            let mut state = state.lock().unwrap();
                            let id = state.create_entry(data).recv().unwrap();
                            let data = state.entries().get(&id).unwrap().clone();
                            state.callbacks.iter_all_mut()
                                .filter(|(ty, _)| **ty == CallbackType::Add)
                                .flat_map(|(_, cbs)| cbs)
                                .for_each(|cb| cb(&data))
                        }
                        ReceivedPacket::EntryUpdate(eu) => {
                            let mut state = state.lock().unwrap();
                            if let Some(entry) = state.entries.get(&eu.entry_id) {
                                if entry.seqnum < eu.entry_seqnum && entry.entry_type() == eu.entry_type {
                                    state.clients.iter()
                                        .filter(|(client_addr, _)| **client_addr != addr)
                                        .for_each(|(_, tx)| tx.unbounded_send(Box::new(eu.clone())).unwrap());
                                    state.update_entry(eu.entry_id, eu.entry_value);
                                    state.entries.get_mut(&eu.entry_id).unwrap().seqnum = eu.entry_seqnum;

                                    let data = state.entries().get(&eu.entry_id).unwrap().clone();
                                    state.callbacks.iter_all_mut()
                                        .filter(|(ty, _)| **ty == CallbackType::Update)
                                        .flat_map(|(_, cbs)| cbs)
                                        .for_each(|cb| cb(&data))
                                }
                            }
                        }
                        ReceivedPacket::EntryFlagsUpdate(efu) => {
                            let mut state = state.lock().unwrap();
                            state.clients.iter()
                                .filter(|(client_addr, _)| **client_addr != addr)
                                .for_each(|(_, tx)| tx.unbounded_send(Box::new(efu)).unwrap());
                            state.update_entry_flags(efu.entry_id, efu.entry_flags);
                        }
                        ReceivedPacket::EntryDelete(ed) => {
                            let mut state = state.lock().unwrap();
                            state.clients.iter()
                                .filter(|(client_addr, _)| **client_addr != addr)
                                .for_each(|(_, tx)| tx.unbounded_send(Box::new(ed)).unwrap());

                            {
                                let data = state.entries().get(&ed.entry_id).unwrap().clone();
                                state.callbacks.iter_all_mut()
                                    .filter(|(ty, _)| **ty == CallbackType::Delete)
                                    .flat_map(|(_, cbs)| cbs)
                                    .for_each(|cb| cb(&data))
                            }

                            state.delete_entry(ed.entry_id);
                        }
                        ReceivedPacket::ClearAllEntries(cea) => {
                            if cea.is_valid() {
                                let mut state = state.lock().unwrap();
                                state.clients.iter()
                                    .filter(|(client_addr, _)| **client_addr != addr)
                                    .for_each(|(_, tx)| tx.unbounded_send(Box::new(cea)).unwrap());
                                state.clear_entries();
                            }
                        }
                        ReceivedPacket::KeepAlive => {}
                        _ => {
                            return Err(err_msg("Invalid packet sent during Connected state"));
                        }
                    }
                    Ok(())
                }
            }
        })
        .map_err(move |_| {
            let mut state = err_state.lock().unwrap();
            state.delete_client(&addr);
            state
                .server_callbacks.iter_all_mut()
                .filter(|(ty, _)| **ty == ServerCallbackType::ClientDisconnected)
                .flat_map(|(_, cbs)| cbs)
                .for_each(|cb| cb(&addr));
//            println!("Closing client with err {}", e);
        }));
}
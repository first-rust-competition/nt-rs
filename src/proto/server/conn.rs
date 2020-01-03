use crate::proto::server::ServerState;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, Receiver};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{Framed, Decoder};
use nt_network::{Packet, ReceivedPacket, NTVersion, ProtocolVersionUnsupported, ServerHello, EntryAssignment, ServerHelloComplete};
use futures_util::StreamExt;
use futures_util::sink::{SinkExt, Sink};
use nt_network::codec::NTCodec;
use std::net::SocketAddr;
use crate::proto::State;
use crate::{EntryData, ServerCallbackType, CallbackType};
use futures_util::stream::Stream;

pub async fn connection(ip: String, state: Arc<Mutex<ServerState>>, close_rx: Receiver<()>) -> crate::Result<()> {
    let mut listener = TcpListener::bind(ip).await?;

    //TODO: integrate close_rx
    loop {
        let (conn, addr) = listener.accept().await?;

        let (tx, rx) = unbounded::<Box<dyn Packet>>();
        state.lock().unwrap().clients.insert(addr, tx);
        tokio::spawn(client_conn(addr, NTCodec.framed(conn), rx, state.clone()));
    }
    Ok(())
}

#[cfg(feature = "websocket")]
pub async fn connection_ws(url: String, state: Arc<Mutex<ServerState>>, close_rx: Receiver<()>) -> crate::Result<()> {
    use crate::proto::ws::{WSCodec, ServerHeaderCallback};
    use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

    let mut listener = TcpListener::bind(url).await?;

    loop {
        let (conn, addr) = listener.accept().await?;

        let conn = match tokio_tungstenite::accept_hdr_async(conn, ServerHeaderCallback).await {
            Ok(conn) => conn,
            Err(e) => {
                println!("Connection from {} rejected. {:?}", addr, e);
                continue;
            }
        };
        let codec = WSCodec::new(conn);

        let (tx, rx) = unbounded::<Box<dyn Packet>>();
        state.lock().unwrap().clients.insert(addr, tx);
        tokio::spawn(client_conn(addr, codec, rx, state.clone()));
    }
}

async fn client_conn<T>(addr: SocketAddr, conn: T, mut packet_rx: UnboundedReceiver<Box<dyn Packet>>, state: Arc<Mutex<ServerState>>) -> crate::Result<()>
    where T: Sink<Box<dyn Packet>> + Stream<Item=crate::Result<ReceivedPacket>> + Send + 'static
{
    let (mut tx, mut rx) = conn.split();

    tokio::spawn(async move {
        while let Some(packet) = packet_rx.next().await {
            let _ = tx.send(packet).await;
        }
    });

    while let Some(packet) = rx.next().await {
        if let Ok(packet) = packet {
            let packet: ReceivedPacket = packet;
            match packet {
                ReceivedPacket::ClientHello(hello) => {
                    if hello.version != NTVersion::V3 {
                        state.lock().unwrap().clients[&addr].unbounded_send(Box::new(ProtocolVersionUnsupported::new(NTVersion::V3))).unwrap();
                        break;
                    }
                    let mut state = state.lock().unwrap();
                    let tx = &state.clients[&addr];
                    tx.unbounded_send(Box::new(ServerHello::new(0, state.server_name.clone()))).unwrap();

                    for (id, entry) in state.entries() {
                        let packet = Box::new(EntryAssignment::new(entry.name.clone(), entry.entry_type(), *id, entry.seqnum, entry.flags, entry.value.clone()));
                        tx.unbounded_send(packet).unwrap();
                    }

                    tx.unbounded_send(Box::new(ServerHelloComplete)).unwrap();
                }
                ReceivedPacket::ClientHelloComplete => {
                    state.lock().unwrap().server_callbacks
                        .iter_all_mut()
                        .filter(|(cb, _)| **cb == ServerCallbackType::ClientConnected)
                        .flat_map(|(_, cbs)| cbs)
                        .for_each(|cb| cb(&addr))
                }
                ReceivedPacket::EntryAssignment(ea) => {
                    if ea.entry_id == 0xFFFF {
                        state.lock().unwrap().create_entry(EntryData::new(ea.entry_name, ea.entry_flags, ea.entry_value));
                    }
                    // should i be evil here? nasal demons are fun
                }
                ReceivedPacket::EntryUpdate(eu) => {
                    let mut state = state.lock().unwrap();
                    if let Some(entry) = state.entries.get_mut(&eu.entry_id) {
                        if eu.entry_seqnum > entry.seqnum && eu.entry_type == entry.entry_type() {
                            entry.value = eu.entry_value.clone();
                        }
                        entry.seqnum += 1;
                        let entry = entry.clone();
                        for tx in state.clients.iter().filter(|(_addr, _)| **_addr != addr)
                            .map(|(_, tx)| tx) {
                            tx.unbounded_send(Box::new(eu.clone())).unwrap();
                        }


                        state.callbacks.iter_all_mut()
                            .filter(|(cb, _)| **cb == CallbackType::Update)
                            .flat_map(|(_, cbs)| cbs)
                            .for_each(|cb| cb(&entry));
                    }
                }
                ReceivedPacket::EntryFlagsUpdate(efu) => {
                    let mut state = state.lock().unwrap();
                    if let Some(entry) = state.entries.get_mut(&efu.entry_id) {
                        entry.flags = efu.entry_flags;

                        for tx in state.clients.iter().filter(|(_addr, _)| **_addr != addr)
                            .map(|(_, tx)| tx) {
                            tx.unbounded_send(Box::new(efu.clone())).unwrap();
                        }
                    }
                }
                ReceivedPacket::EntryDelete(ed) => {
                    let mut state = state.lock().unwrap();
                    let entry = state.entries.remove(&ed.entry_id).unwrap();

                    for tx in state.clients.iter().filter(|(_addr, _)| **_addr != addr)
                        .map(|(_, tx)| tx) {
                        tx.unbounded_send(Box::new(ed.clone())).unwrap();
                    }

                    state.callbacks.iter_all_mut()
                        .filter(|(cb, _)| **cb == CallbackType::Delete)
                        .flat_map(|(_, cbs)| cbs)
                        .for_each(|cb| cb(&entry));
                }
                ReceivedPacket::ClearAllEntries(cea) => {
                    if cea.is_valid() {
                        let mut state = state.lock().unwrap();
                        state.entries.clear();
                        for tx in state.clients.iter().filter(|(_addr, _)| **_addr != addr)
                            .map(|(_, tx)| tx) {
                            tx.unbounded_send(Box::new(cea.clone())).unwrap();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let mut state = state.lock().unwrap();
    state.clients.remove(&addr);

    state.server_callbacks
        .iter_all_mut()
        .filter(|(cb, _)| **cb == ServerCallbackType::ClientDisconnected)
        .flat_map(|(_, cbs)| cbs)
        .for_each(|cb| cb(&addr));
    Ok(())
}
use crate::proto::client::ClientState;
use nt_network::{Packet, ReceivedPacket, ClientHelloComplete, ClientHello, NTVersion};
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio_util::codec::Decoder;
use futures_util::StreamExt;
use futures_util::sink::SinkExt;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, Receiver};
use nt_network::codec::NTCodec;
use crate::{EntryData, CallbackType};
use crate::proto::State;
use failure::bail;
use futures_util::future::Either;
use futures_util::stream::select;
#[cfg(feature = "websocket")]
use url::Url;
#[cfg(feature = "websocket")]
use tokio_tungstenite::tungstenite::handshake::client::Request;
#[cfg(feature = "websocket")]
use std::borrow::Cow;
#[cfg(feature = "websocket")]
use crate::proto::ws::WSCodec;

pub async fn connection(state: Arc<Mutex<ClientState>>, mut packet_rx: UnboundedReceiver<Box<dyn Packet>>, ip: String, client_name: String, ready_tx: UnboundedSender<()>, close_rx: Receiver<()>) -> crate::Result<()> {
    let conn = TcpStream::connect(ip).await?;
    let (mut tx, rx) = NTCodec.framed(conn).split();
    tokio::spawn(async move {
        tx.send(Box::new(ClientHello::new(NTVersion::V3, client_name))).await.unwrap();
        while let Some(packet) = packet_rx.next().await {
            tx.send(packet).await.unwrap();
        }
    });

    let mut rx = select(rx.map(Either::Left), close_rx.map(Either::Right));

    while let Some(msg) = rx.next().await {
        match msg {
            Either::Left(packet) => if let Ok(packet) = packet {
                match packet {
                    ReceivedPacket::ServerHelloComplete => {
                        ready_tx.unbounded_send(()).unwrap();
                        state.lock().unwrap().packet_tx.unbounded_send(Box::new(ClientHelloComplete)).unwrap();
                    }
                    packet @ _ => handle_packet(packet, &state)?
                }
            },
            Either::Right(_) => return Ok(())
        }
    }
    Ok(())
}

#[cfg(feature = "websocket")]
pub async fn connection_ws(state: Arc<Mutex<ClientState>>, mut packet_rx: UnboundedReceiver<Box<dyn Packet>>, url: String, client_name: String, ready_tx: UnboundedSender<()>, close_rx: Receiver<()>) -> crate::Result<()> {
    let url = Url::parse(&url).unwrap();
    let mut req = Request {
        url,
        extra_headers: None,
    };
    req.add_protocol(Cow::Borrowed("NetworkTables"));
    println!("Trying to connect: {:?}", req);
    let (sock, resp) = tokio_tungstenite::connect_async(req).await?;

    println!("Got ws response {:?}", resp);

    let (mut tx, rx) = WSCodec::new(sock).split();

    tokio::spawn(async move {
        tx.send(Box::new(ClientHello::new(NTVersion::V3, client_name))).await.unwrap();
        while let Some(packet) = packet_rx.next().await {
            tx.send(packet).await.unwrap();
        }
    });

    let mut rx = select(rx.map(Either::Left), close_rx.map(Either::Right));

    while let Some(msg) = rx.next().await {
        match msg {
            Either::Left(packet) => {
                match packet {
                    Ok(packet) => match packet {
                        ReceivedPacket::ServerHelloComplete => {
                            ready_tx.unbounded_send(()).unwrap();
                            state.lock().unwrap().packet_tx.unbounded_send(Box::new(ClientHelloComplete)).unwrap();
                        }
                        packet @ _ => handle_packet(packet, &state)?
                    }
                    Err(e) => return Err(e.into())
                }
            },
            Either::Right(_) => return Ok(()),
        }
    }
    Ok(())
}

fn handle_packet(packet: ReceivedPacket, state: &Arc<Mutex<ClientState>>) -> crate::Result<()> {
    match packet {
        ReceivedPacket::EntryAssignment(ea) => {
            let mut state = state.lock().unwrap();
            if let Some(mut tx) = state.pending_entries.remove(&ea.entry_name) {
                tx.try_send(ea.entry_id).unwrap();
            }

            let data = EntryData::new(ea.entry_name, ea.entry_flags, ea.entry_value);
            state.callbacks.iter_all_mut()
                .filter(|(cb, _)| **cb == CallbackType::Add)
                .flat_map(|(_, cbs)| cbs)
                .for_each(|cb| cb(&data));
            state.entries.insert(ea.entry_id, data);
        }
        ReceivedPacket::KeepAlive => {}
        ReceivedPacket::ClientHello(_) => {}
        ReceivedPacket::ProtocolVersionUnsupported(pvu) => {
            bail!("Server does not support NTv3. Supported protocol: {}", pvu.supported_version);
        }
        ReceivedPacket::ServerHello(_) => {}
        ReceivedPacket::ClientHelloComplete => {}
        ReceivedPacket::EntryUpdate(eu) => {
            let mut state = state.lock().unwrap();
            if let Some(entry) = state.entries.get_mut(&eu.entry_id) {
                entry.value = eu.entry_value;
                entry.seqnum = eu.entry_seqnum;

                // Gross but necessary to ensure unique mutable borrows
                let entry = entry.clone();

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
            }
        }
        ReceivedPacket::EntryDelete(ed) => {
            let mut state = state.lock().unwrap();
            if let Some(data) = state.entries.remove(&ed.entry_id) {
                state.callbacks.iter_all_mut()
                    .filter(|(cb, _)| **cb == CallbackType::Delete)
                    .flat_map(|(_, cbs)| cbs)
                    .for_each(|cb| cb(&data));
            }
        }
        ReceivedPacket::ClearAllEntries(cea) => {
            if cea.is_valid() {
                state.lock().unwrap().clear_entries();
            }
        }
        _ => {}
    }
    Ok(())
}

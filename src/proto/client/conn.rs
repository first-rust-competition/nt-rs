use crate::proto::client::ClientState;
#[cfg(feature = "websocket")]
use crate::proto::ws::WSCodec;
use crate::proto::State;
use crate::{CallbackType, ConnectionCallbackType, EntryData};
use futures_channel::mpsc::{Receiver, UnboundedReceiver, UnboundedSender};
use futures_util::future::Either;
use futures_util::sink::SinkExt;
use futures_util::stream::select;
use futures_util::StreamExt;
use nt_network::codec::NTCodec;
use nt_network::{ClientHello, ClientHelloComplete, KeepAlive, NTVersion, Packet, ReceivedPacket};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpStream;
#[cfg(feature = "websocket")]
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_util::codec::Decoder;
#[cfg(feature = "websocket")]
use url::Url;
use crate::error::Error;

pub async fn connection(
    state: Arc<Mutex<ClientState>>,
    packet_rx: UnboundedReceiver<Box<dyn Packet>>,
    ready_tx: UnboundedSender<()>,
    close_rx: Receiver<()>,
) -> crate::Result<()> {
    let (ip, client_name) = {
        let state = state.lock().unwrap();
        (state.ip.clone(), state.name.clone())
    };
    let conn = TcpStream::connect(ip).await?;
    let addr = conn.local_addr().unwrap();
    let (mut tx, mut rx) = NTCodec.framed(conn).split();

    let rx_state = state.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.next().await {
            if let Ok(packet) = msg {
                match packet {
                    ReceivedPacket::ServerHelloComplete => {
                        ready_tx.unbounded_send(()).unwrap();
                        let mut state = rx_state.lock().unwrap();
                        state
                            .connection_callbacks
                            .iter_all_mut()
                            .filter(|(cb, _)| **cb == ConnectionCallbackType::ClientConnected)
                            .flat_map(|(_, cbs)| cbs)
                            .for_each(|cb| cb(&addr));
                        state.connected = true;
                        state
                            .packet_tx
                            .unbounded_send(Box::new(ClientHelloComplete))
                            .unwrap();
                    }
                    packet => handle_packet(packet, &rx_state).unwrap(),
                }
            }
        }
    });

    let tick_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::new(1, 0));

        loop {
            let _ = tick_state
                .lock()
                .unwrap()
                .packet_tx
                .unbounded_send(Box::new(KeepAlive));
            interval.tick().await;
        }
    });

    let mut rx = select(packet_rx.map(Either::Left), close_rx.map(Either::Right));

    let tx_state = state.clone();
    tx.send(Box::new(ClientHello::new(NTVersion::V3, client_name)))
        .await
        .unwrap();
    while let Some(msg) = rx.next().await {
        match msg {
            Either::Left(packet) => match tx.send(packet).await {
                Ok(_) => {}
                Err(e) => {
                    if let Ok(e) = e.downcast::<std::io::Error>() {
                        if e.kind() == std::io::ErrorKind::BrokenPipe {
                            // connection terminated
                            let mut state = tx_state.lock().unwrap();
                            state
                                .connection_callbacks
                                .iter_all_mut()
                                .filter(|(cb, _)| {
                                    **cb == ConnectionCallbackType::ClientDisconnected
                                })
                                .flat_map(|(_, cbs)| cbs)
                                .for_each(|cb| cb(&addr));
                            state.connected = false;
                            return Ok(());
                        }
                    }
                }
            },
            Either::Right(_) => return Ok(()),
        }
    }

    Ok(())
}

#[cfg(feature = "websocket")]
pub async fn connection_ws(
    state: Arc<Mutex<ClientState>>,
    mut packet_rx: UnboundedReceiver<Box<dyn Packet>>,
    ready_tx: UnboundedSender<()>,
    close_rx: Receiver<()>,
) -> crate::Result<()> {
    use tokio_tungstenite::tungstenite::http::HeaderValue;
    let (url, client_name) = {
        let state = state.lock().unwrap();
        (state.ip.clone(), state.name.clone())
    };

    let _url = Url::parse(&url).unwrap();

    let domain = _url.host_str().unwrap();
    let port = _url.port().unwrap();
    let addr = format!("{}:{}", domain, port).parse().unwrap();

    let req = Request::get(url)
        .header("Sec-WebSocket-Protocol", HeaderValue::from_str("NetworkTables").unwrap()).body(()).unwrap();
    let (sock, _resp) = tokio_tungstenite::connect_async(req).await?;
    println!("Connected to remote.");

    let (mut tx, rx) = WSCodec::new(sock).split();

    tokio::spawn(async move {
        tx.send(Box::new(ClientHello::new(NTVersion::V3, client_name)))
            .await
            .unwrap();
        while let Some(packet) = packet_rx.next().await {
            tx.send(packet).await.unwrap();
        }
    });

    let tick_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::new(1, 0));

        loop {
            let _ = tick_state
                .lock()
                .unwrap()
                .packet_tx
                .unbounded_send(Box::new(KeepAlive));
            interval.tick().await;
        }
    });

    let mut rx = select(rx.map(Either::Left), close_rx.map(Either::Right));

    while let Some(msg) = rx.next().await {
        match msg {
            Either::Left(packet) => match packet {
                Ok(packet) => match packet {
                    ReceivedPacket::ServerHelloComplete => {
                        ready_tx.unbounded_send(()).unwrap();
                        let mut state = state.lock().unwrap();
                        state
                            .packet_tx
                            .unbounded_send(Box::new(ClientHelloComplete))
                            .unwrap();
                        state.connected = true;
                    }
                    packet @ _ => handle_packet(packet, &state)?,
                },
                Err(_) => {
                    let mut state = state.lock().unwrap();
                    state
                        .connection_callbacks
                        .iter_all_mut()
                        .filter(|(cb, _)| **cb == ConnectionCallbackType::ClientDisconnected)
                        .flat_map(|(_, cbs)| cbs)
                        .for_each(|cb| cb(&addr));
                    state.connected = false;
                    return Ok(());
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
            state
                .callbacks
                .iter_all_mut()
                .filter(|(cb, _)| **cb == CallbackType::Add)
                .flat_map(|(_, cbs)| cbs)
                .for_each(|cb| cb(&data));
            state.entries.insert(ea.entry_id, data);
        }
        ReceivedPacket::KeepAlive => {}
        ReceivedPacket::ClientHello(_) => {}
        ReceivedPacket::ProtocolVersionUnsupported(pvu) => {
            return Err(Error::UnsupportedProtocolVersion { supported_version: NTVersion::from_u16(pvu.supported_version).unwrap() });
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

                state
                    .callbacks
                    .iter_all_mut()
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
                state
                    .callbacks
                    .iter_all_mut()
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

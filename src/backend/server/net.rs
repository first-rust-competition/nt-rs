use crate::backend::server::client::ConnectedClient;
use crate::backend::server::{NTServer, ServerMessage, MAX_BATCHING_SIZE};
use crate::proto::codec::NTSocket;
use crate::proto::prelude::{MessageBody, NTMessage};
use itertools::Itertools;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, watch};
use tokio_tungstenite::stream::Stream;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};
use tokio_tungstenite::{tungstenite, WebSocketStream};
use futures::future::{select, Either};
use tokio::stream::StreamExt;

pub async fn tcp_loop(
    state: Arc<Mutex<NTServer>>,
    tx: Sender<ServerMessage>,
    addr: impl ToSocketAddrs,
    mut close_rx: watch::Receiver<u8>,
) -> anyhow::Result<()> {
    let mut srv = TcpListener::bind(addr).await?;
    log::info!("Server bound at {}", srv.local_addr().unwrap());

    loop {
        tokio::select! {
            Ok((sock, addr)) = srv.accept() => {
                log::info!("Unsecure TCP connection at {}", addr);
                let cid = rand::random::<u32>();
                let sock = try_accept(sock).await;

                if let Ok(sock) = sock {
                    log::info!("Client assigned ID {}", cid);
                    let client = ConnectedClient::new(NTSocket::new(sock), tx.clone(), cid, &state);
                    state.lock().await.clients.insert(cid, client);
                    tokio::spawn(update_new_client(cid, state.clone()));
                }
            }
            close = close_rx.recv() => {
                if close == Some(1) {
                    log::info!("TCP accept task shutting down...");
                    break;
                }
            }
        }
    }

    let mut state = state.lock().await;
    for client in state.clients.values_mut() {
        client.send_message(NTMessage::Close).await;
    }
    log::info!("All clients notified. TCP task shut down.");
    Ok(())
}

async fn try_accept(stream: TcpStream) -> tungstenite::Result<WebSocketStream<TcpStream>> {
    tokio_tungstenite::accept_hdr_async(stream, |req: &Request, mut res: Response| {
        let ws_proto = req.headers().iter().find(|(hdr, _)| hdr.to_string().eq_ignore_ascii_case("sec-websocket-protocol"));

        match ws_proto.map(|(_, s)| s.to_str().unwrap()) {
            Some(s) if s.contains("networktables.first.wpi.edu") => {
                res.headers_mut().insert("Sec-WebSocket-Protocol", HeaderValue::from_static("networktables.first.wpi.edu"));
                Ok(res)
            }
            _ => {
                log::error!("Rejecting client that did not specify correct subprotocol");
                Err(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Some("Protocol 'networktables.first.wpi.edu' required to communicate with this server".to_string())).unwrap())
            }
        }
    }).await
}

async fn update_new_client(id: u32, state: Arc<Mutex<NTServer>>) {
    let mut state = state.lock().await;
    let state = state.deref_mut();
    let client = state.clients.get_mut(&id).unwrap();

    let batches = state
        .topics
        .values()
        .map(|value| client.announce(value).into_message())
        .chunks(MAX_BATCHING_SIZE)
        .into_iter()
        .map(|batch| NTMessage::Text(batch.collect()))
        .collect::<Vec<NTMessage>>();

    for msg in batches {
        client.send_message(msg).await;
    }
}

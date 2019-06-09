use tokio::prelude::*;
use nt_network::{Result, Packet, EntryAssignment};
use tokio::codec::Decoder;
use tokio::net::TcpListener;
use nt_network::codec::NTCodec;
use crate::nt::EntryData;
use crate::nt::callback::*;
use super::State;

mod conn;

use self::conn::handle_client;
use std::collections::HashMap;
use std::net::SocketAddr;
use futures::sync::mpsc::{self, UnboundedSender};
use futures::future::Either;
use failure::err_msg;
use std::sync::{Arc, Mutex};
use crossbeam_channel::{Receiver, unbounded};
use nt_network::types::EntryValue;
use std::thread;
use multimap::MultiMap;
#[cfg(feature = "websocket")]
use websocket::server::NoTlsAcceptor;
#[cfg(feature = "websocket")]
use websocket::r#async::server::Server;
#[cfg(feature = "websocket")]
use super::ws::WSCodec;

pub trait NetworkBackend {}

impl NetworkBackend for TcpListener {}

#[cfg(feature = "websocket")]
impl NetworkBackend for Server<NoTlsAcceptor> {}

pub struct ServerCore<T: NetworkBackend>
{
    sock: T,
    state: Arc<Mutex<ServerState>>,
    close_rx: mpsc::Receiver<()>,
}

pub struct ServerState {
    server_name: String,
    clients: HashMap<SocketAddr, UnboundedSender<Box<dyn Packet>>>,
    entries: HashMap<u16, EntryData>,
    callbacks: MultiMap<CallbackType, Box<Action>>,
    server_callbacks: MultiMap<ServerCallbackType, Box<ServerAction>>,
    next_id: u16,
}

impl ServerState {
    pub fn new(ip: String, server_name: String, close_rx: mpsc::Receiver<()>) -> Arc<Mutex<ServerState>> {
        let _self = Arc::new(Mutex::new(ServerState { clients: HashMap::new(), server_name, entries: HashMap::new(), next_id: 0, callbacks: MultiMap::new(), server_callbacks: MultiMap::new() }));
        let state = _self.clone();
        thread::spawn(move || {
            let sock = TcpListener::bind(&ip.parse().unwrap()).unwrap();
            let core = ServerCore::new(state, sock, close_rx).unwrap();
            core.run().map_err(drop)
        });

        _self
    }

    #[cfg(feature = "websocket")]
    pub fn new_ws(ip: String, server_name: String, close_rx: mpsc::Receiver<()>) -> Arc<Mutex<ServerState>> {
        let _self = Arc::new(Mutex::new(ServerState { clients: HashMap::new(), server_name, entries: HashMap::new(), next_id: 0, callbacks: MultiMap::new(), server_callbacks: MultiMap::new() }));
        let state = _self.clone();
        thread::spawn(move || {
            use websocket::r#async::Server;
            use tokio::reactor::Handle;

            let handle = Handle::default();
            let server = Server::bind(ip, &handle).unwrap();
            let core = ServerCore::new_ws(state, server, close_rx).unwrap();
            core.run_ws().map_err(drop)
        });

        _self
    }

    #[cfg(feature = "websocket")]
    pub fn new_both(tcp_ip: String, ws_ip: String, server_name: String, close_rx: mpsc::Receiver<()>) -> Arc<Mutex<ServerState>> {
        let _self = Arc::new(Mutex::new(ServerState { clients: HashMap::new(), server_name, entries: HashMap::new(), next_id: 0, callbacks: MultiMap::new(), server_callbacks: MultiMap::new() }));
        let (ws_tx, ws_rx) = mpsc::channel::<()>(1);
        let (tcp_tx, tcp_rx) = mpsc::channel::<()>(1);

        let state = _self.clone();

        thread::spawn(move || {
            let close_rx = close_rx;
            use websocket::r#async::Server;
            use tokio::reactor::Handle;

            let handle = Handle::default();

            let ws_server = Server::bind(ws_ip, &handle).unwrap();
            let ws_core = ServerCore::new_ws(state.clone(), ws_server, ws_rx).unwrap();

            let tcp_server = TcpListener::bind(&tcp_ip.parse().unwrap()).unwrap();
            let tcp_core = ServerCore::new(state.clone(), tcp_server, tcp_rx).unwrap();

            tokio::run(futures::lazy(|| Ok(()))
                .and_then(move |_| ws_core.spawn_ws())
                .and_then(move |_| tcp_core.spawn())
                .map_err(drop)
                .and_then(move |_| close_rx.into_future().map_err(drop))
                .and_then(move |_| ws_tx.send(()).map_err(drop))
                .and_then(move |_| tcp_tx.send(()).map_err(drop))
                .then(|_| Ok(())));
        });

        _self
    }

    pub fn add_client(&mut self, socket: SocketAddr, ch: UnboundedSender<Box<dyn Packet>>) {
        self.clients.insert(socket, ch);
    }

    pub fn entries(&self) -> &HashMap<u16, EntryData> {
        &self.entries
    }

    pub fn delete_client(&mut self, socket: &SocketAddr) {
        self.clients.remove(socket);
    }

    pub fn add_server_callback(&mut self, callback_type: ServerCallbackType, action: impl FnMut(&SocketAddr) + Send + 'static) {
        self.server_callbacks.insert(callback_type, Box::new(action));
    }
}

impl State for ServerState {
    fn entries(&self) -> &HashMap<u16, EntryData> {
        &self.entries
    }

    fn entries_mut(&mut self) -> &mut HashMap<u16, EntryData> {
        &mut self.entries
    }

    fn create_entry(&mut self, data: EntryData) -> Receiver<u16> {
        let packet = EntryAssignment::new(data.name.clone(), data.entry_type(), self.next_id, data.seqnum, data.flags, data.value.clone());

        self.entries.insert(self.next_id, data);

        for tx in self.clients.values() {
            tx.unbounded_send(Box::new(packet.clone())).unwrap();
        }

        let (tx, rx) = unbounded::<u16>();
        tx.send(self.next_id).unwrap();
        self.next_id += 1;
        if self.next_id == 0xFFFF {
            self.next_id = 0; // Roll over early, 0xFFFF is reserved for entry assignments by clients
        }
        rx
    }

    fn delete_entry(&mut self, id: u16) {
        self.entries.remove(&id);
    }

    /// Updates the server internal value for the entry of the given id
    /// The server is expected to handle the rebroadcast of the given packet, as
    /// this has no concept of what client sent the message
    fn update_entry(&mut self, id: u16, new_value: EntryValue) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.seqnum += 1;
            entry.value = new_value;
        }
    }

    fn update_entry_flags(&mut self, id: u16, flags: u8) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.flags = flags;
        }
    }

    fn clear_entries(&mut self) {
        self.entries.clear();
        unimplemented!()
    }

    fn add_callback(&mut self, callback_type: CallbackType, action: impl FnMut(&EntryData) + Send + 'static) {
        self.callbacks.insert(callback_type, Box::new(action));
    }
}

impl ServerCore<TcpListener> {
    pub fn new(state: Arc<Mutex<ServerState>>, sock: TcpListener, close_rx: mpsc::Receiver<()>) -> Result<ServerCore<TcpListener>> {
        Ok(ServerCore { sock, state, close_rx })
    }

    pub fn spawn(self) -> Result<()> {
        tokio::spawn(self.gen_future());
        Ok(())
    }

    pub fn run(self) -> Result<()> {
        tokio::run(self.gen_future());
        Ok(())
    }

    fn gen_future(self) -> impl Future<Item=(), Error=()> {
        let state = self.state.clone();
        let close_rx = self.close_rx;
        self.sock.incoming().map(|conn| (conn.peer_addr().unwrap(), NTCodec.framed(conn)))
            .map(Either::A)
            .map_err(|e| failure::Error::from(e))
            .select(close_rx.map(Either::B).map_err(|_| err_msg("unreachable")))
            .for_each(move |msg| {
                match msg {
                    Either::A((addr, codec)) => {
                        let (tx, rx) = codec.split();
                        handle_client(addr, tx, rx, state.clone());
                        Ok(())
                    }
                    Either::B(_) => Err(err_msg("NetworkTables dropped"))
                }
            }).map_err(|e| println!("Error from peer socket: {}", e))
    }
}


#[cfg(feature = "websocket")]
impl ServerCore<Server<NoTlsAcceptor>> {
    pub fn new_ws(state: Arc<Mutex<ServerState>>, sock: Server<NoTlsAcceptor>, close_rx: mpsc::Receiver<()>) -> Result<ServerCore<Server<NoTlsAcceptor>>> {
        Ok(ServerCore { sock, state, close_rx })
    }

    pub fn run_ws(self) -> Result<()> {
        tokio::run(self.gen_future_ws());
        Ok(())
    }

    pub fn spawn_ws(self) -> Result<()> {
        tokio::spawn(self.gen_future_ws());
        Ok(())
    }

    fn gen_future_ws(self) -> impl Future<Item=(), Error=()> {
        let state = self.state.clone();
        let close_rx = self.close_rx;
        self.sock.incoming()
            .map_err(|e| err_msg(format!("{:?}", e.error)))
            .map(Either::A)
            .select(close_rx.map(Either::B).map_err(|_| err_msg("unreachable")))
            .for_each(move |msg| {
                match msg {
                    Either::A((upgrade, addr)) => {
                        let state = state.clone(); // tokio semantics dum
                        if upgrade.protocols().contains(&"NetworkTables".to_string()) {
                            tokio::spawn(upgrade.use_protocol("NetworkTables")
                                .accept()
                                .and_then(move |(conn, _)| {
                                    let codec = WSCodec::new(conn);
                                    let (tx, rx) = codec.split();
                                    handle_client(addr, tx, rx, state.clone());
                                    Ok(())
                                }).map_err(drop).map(drop));
                        } else {
                            tokio::spawn(upgrade.reject().then(|_| Ok(())));
                        }
                        Ok(())
                    }
                    Either::B(_) => Err(err_msg("NetworkTables dropped"))
                }
            }).map_err(drop).map(drop)
    }
}

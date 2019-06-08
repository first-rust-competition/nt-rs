use nt_network::{ReceivedPacket, Packet, Result, ClientHello, NTVersion, EntryAssignment, EntryDelete, EntryUpdate, EntryFlagsUpdate, ClearAllEntries};
use tokio::prelude::*;
use tokio::codec::{Decoder, Framed};
use nt_network::codec::NTCodec;
use futures::sync::mpsc::{self, UnboundedReceiver, Receiver, channel, UnboundedSender, unbounded};
use failure::err_msg;
use crossbeam_channel::Receiver as CrossbeamReceiver;
use crate::nt::EntryData;
use crate::nt::callback::*;

use super::State;
#[cfg(feature = "websocket")]
use super::ws::WSCodec;

mod conn;

use self::conn::{send_packets, poll_socket};
use std::collections::HashMap;
use multimap::MultiMap;
use crossbeam_channel::Sender;
use nt_network::types::EntryValue;
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use std::thread;
#[cfg(feature = "websocket")]
use websocket::r#async::Client;
use std::time::Duration;

pub struct ClientCore<T>
    where T: Stream<Item=ReceivedPacket, Error=failure::Error> + Sink<SinkItem=Box<dyn Packet>, SinkError=failure::Error> + Send + 'static
{
    //    codec: Framed<TcpStream, NTCodec>,
    codec: T,
    tx_channel: UnboundedReceiver<Box<dyn Packet>>,
    client_name: String,
    state: Arc<Mutex<ClientState>>,
    close_rx: Receiver<()>,
}

impl ClientCore<Framed<TcpStream, NTCodec>> {
    pub fn new(conn: TcpStream, state: Arc<Mutex<ClientState>>, client_name: String, tx_channel: UnboundedReceiver<Box<dyn Packet>>, close_rx: Receiver<()>) -> Result<ClientCore<Framed<TcpStream, NTCodec>>> {
        let codec = NTCodec.framed(conn);
        Ok(ClientCore { codec, state, tx_channel, client_name, close_rx })
    }
}

#[cfg(feature = "websocket")]
impl ClientCore<WSCodec> {
    pub fn new_ws(conn: Client<TcpStream>, state: Arc<Mutex<ClientState>>, client_name: String, tx_channel: UnboundedReceiver<Box<dyn Packet>>, close_rx: Receiver<()>) -> Result<ClientCore<WSCodec>> {
        let codec = WSCodec::new(conn);
        Ok(ClientCore { codec, state, tx_channel, client_name, close_rx })
    }
}

impl<T> ClientCore<T>
    where T: Stream<Item=ReceivedPacket, Error=failure::Error> + Sink<SinkItem=Box<dyn Packet>, SinkError=failure::Error> + Send + 'static
{
    pub fn run(self, packet_tx: UnboundedSender<Box<dyn Packet>>, wait_tx: crossbeam_channel::Sender<()>) -> Result<()> {
        let (tx, rx) = self.codec.split();
        let outbound_packets_ch = self.tx_channel;
        let close_rx = self.close_rx;

        let (handshake_tx, handshake_rx) = channel::<()>(1);

        let packet = Box::new(ClientHello::new(NTVersion::V3, self.client_name.clone()));
        tokio::spawn(tx.send(packet)
            .map_err(drop)
            .and_then(|tx| send_packets(tx, outbound_packets_ch, handshake_rx, packet_tx)));

        tokio::spawn(poll_socket(rx.map_err(drop), handshake_tx, self.state.clone(), close_rx, wait_tx));

        Ok(())
    }
}

pub struct ClientState {
    entries: HashMap<u16, EntryData>,
    tx_channel: UnboundedSender<Box<dyn Packet>>,
    callbacks: MultiMap<CallbackType, Box<Action>>,
    pending_entries: HashMap<String, Sender<u16>>,
}

impl ClientState {
    pub fn new(ip: String, client_name: String, close_rx: mpsc::Receiver<()>) -> Result<Arc<Mutex<ClientState>>> {
        let sock = TcpStream::connect(&ip.parse().unwrap()).map_err(drop);
        let (tx, rx) = unbounded::<Box<dyn Packet>>();

        let (wait_tx, wait_rx) = crossbeam_channel::unbounded::<()>();

        let mut _self = Arc::new(Mutex::new(ClientState { entries: HashMap::new(), tx_channel: tx.clone(), pending_entries: HashMap::new(), callbacks: MultiMap::new() }));

        let state = _self.clone();
        let core_tx = tx;
        thread::spawn(move || {
            tokio::run(sock.and_then(move |stream| {
                let core = ClientCore::new(stream, state, client_name, rx, close_rx).unwrap();
                core.run(core_tx, wait_tx).map_err(drop)
            }));
        });

        // Wait for the connection to be fully established
        match wait_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(_) => Ok(_self),
            Err(_) => Err(err_msg("Unable to connect to server"))
        }
    }

    #[cfg(feature = "websocket")]
    pub fn new_ws(ip: String, client_name: String, close_rx: mpsc::Receiver<()>) -> Result<Arc<Mutex<ClientState>>> {
        use websocket::ClientBuilder;
        let client = ClientBuilder::new(&ip)
            .unwrap()
            .add_protocol("NetworkTables")
            .async_connect_insecure();

        let (tx, rx) = unbounded::<Box<dyn Packet>>();
        let (wait_tx, wait_rx) = crossbeam_channel::unbounded::<()>();

        let mut _self = Arc::new(Mutex::new(ClientState { entries: HashMap::new(), tx_channel: tx.clone(), pending_entries: HashMap::new(), callbacks: MultiMap::new() }));

        let state = _self.clone();
        thread::spawn(move || {
            tokio::run(client.and_then(move |(stream, _)| {
                let core = ClientCore::new_ws(stream, state, client_name, rx, close_rx).unwrap();
                core.run(tx, wait_tx).unwrap();
                Ok(())
            }).map_err(drop));
        });

        match wait_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(_) => Ok(_self),
            Err(_) => Err(err_msg("Unable to connect to server"))
        }
    }
}

impl State for ClientState {
    fn entries(&self) -> &HashMap<u16, EntryData> {
        &self.entries
    }

    fn entries_mut(&mut self) -> &mut HashMap<u16, EntryData> {
        &mut self.entries
    }

    fn create_entry(&mut self, data: EntryData) -> CrossbeamReceiver<u16> {
        let (tx, rx) = crossbeam_channel::unbounded::<u16>();
        self.pending_entries.insert(data.name.clone(), tx);

        self.tx_channel.unbounded_send(Box::new(EntryAssignment::new(data.name, data.value.entry_type(),
                                                                     0xFFFF, 1, data.flags, data.value)))
            .unwrap();

        rx
    }

    fn delete_entry(&mut self, id: u16) {
        self.tx_channel.unbounded_send(Box::new(EntryDelete::new(id))).unwrap();
    }

    fn update_entry(&mut self, id: u16, new_value: EntryValue) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.seqnum += 1;
            entry.value = new_value.clone();
            self.tx_channel.unbounded_send(Box::new(EntryUpdate::new(id, entry.seqnum, entry.entry_type(), new_value)))
                .unwrap();
        }
    }

    fn update_entry_flags(&mut self, id: u16, flags: u8) {
        self.tx_channel.unbounded_send(Box::new(EntryFlagsUpdate::new(id, flags)))
            .unwrap();
    }

    fn clear_entries(&mut self) {
        self.tx_channel.unbounded_send(Box::new(ClearAllEntries::new())).unwrap();
    }

    fn add_callback(&mut self, callback_type: CallbackType, action: impl FnMut(&EntryData) + Send + 'static) {
        self.callbacks.insert(callback_type, Box::new(action));
    }
}

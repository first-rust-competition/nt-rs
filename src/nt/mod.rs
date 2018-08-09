use std::net::SocketAddr;
use futures::{Future, Poll};
use futures::future::{ok, Either};
use futures::sync::mpsc::{Sender, Receiver};
use tokio::prelude::*;
use tokio::net::TcpStream;
use tokio_codec::{Decoder, Framed, FramedRead, LinesCodec};
use tokio::io::stdin;

use nt_packet::ClientMessage;
use proto::codec::NTCodec;
use proto::client::*;
use proto::*;

use std::sync::{Arc, Mutex};
use std::io::Error;

pub mod state;
mod handler;

use self::handler::*;
use state::*;

/// Core struct representing a connection to a NetworkTables server
pub struct NetworkTables {
    /// Contains the initial connection future with a reference to the framed NT codec
    handle: Box<Future<Item=Framed<TcpStream, NTCodec>, Error=()> + Send>,
}

/// Implementation of Future delegating to `self.handle`
impl Future for NetworkTables {
    type Item = Framed<TcpStream, NTCodec>;
    type Error = ();

    fn poll(&mut self) -> Poll<Framed<TcpStream, NTCodec>, ()> {
        self.handle.poll()
    }
}

impl NetworkTables {
    /// Performs the initial connection process to the given `target`.
    /// Assumes that target is a valid, running NetworkTables server.
    /// Returns a `NetworkTables` ready to be started on the tokio runtime to initialize the connection
    pub fn connect(client_name: &'static str, target: &SocketAddr, state: Arc<Mutex<State>>) -> NetworkTables {
        state.lock().unwrap().set_state(ConnectionState::Connecting);

        // Internal handle of the struct
        let future = TcpStream::connect(target)
            .and_then(move |sock| {
                // Once we have the socket, frame it with the NT codec and send the first packet to begin the connection
                let codec = NTCodec.framed(sock);
                codec.send(Box::new(ClientHello::new(::NT_PROTOCOL_REV, client_name)))
            })
            .map_err(|_| ());

        NetworkTables {
            handle: Box::new(future),
        }
    }
}

pub fn send_packets(tx: impl Sink<SinkItem=Box<ClientMessage>, SinkError=Error>, rx: Receiver<Box<ClientMessage>>) -> impl Future<Item=(), Error=()> {
    rx
        .map_err(|_| ())
        .fold(tx, |tx, packet| tx.send(packet).map_err(|_| ()))
        .then(|_| Ok(()))
}

pub fn poll_stdin(state: Arc<Mutex<State>>, tx: Sender<Box<ClientMessage>>) -> impl Future<Item=(), Error=()> {
    FramedRead::new(stdin(), LinesCodec::new())
        .map_err(|_| ())
        .fold(tx, move |tx, msg| {
            match handle_user_input(msg, state.clone()) {
                Some(packet) => Either::A(tx.send(packet).map_err(|_|())),
                None => Either::B(ok(tx))
            }
        })
        .then(|_| Ok(()))
}

/// Function containing the future for polling the remote peer for new packets
/// Expects to be started with `tokio::spawn`
/// Mutates the `state` as packets are received
pub fn poll_socket(state: Arc<Mutex<State>>, rx: impl Stream<Item=Packet, Error=Error>, tx: Sender<Box<ClientMessage>>) -> impl Future<Item=(), Error=()> {

    // This function will be called as new packets arrive. Has to be `fold` to maintain ownership over the write half of our codec
    rx
        .map_err(|_| ())
        .fold(tx, move |tx, packet| {
            match handle_packet(packet, state.clone()) {
                Some(packet) => Either::A(tx.send(packet).map_err(|_|())),
                None => Either::B(ok(tx))
            }
        })
        .then(|_| Ok(()))
}



use std::net::SocketAddr;
use futures::{Future, Poll};
use futures::future::{ok, Either};
use tokio::prelude::*;
use tokio::net::TcpStream;
use tokio_codec::{Decoder, Framed};
use tokio::timer::Interval;
use tokio;

use std::time::{Instant, Duration};

use proto::codec::NTCodec;
use proto::client::*;
use proto::*;
use proto::server::*;
use proto::types::*;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub mod state;
use state::*;

pub struct NetworkTables {
    state: Arc<Mutex<State>>,
    handle: Box<Future<Item=Framed<TcpStream, NTCodec>, Error=()> + Send>,
}

impl Future for NetworkTables {
    type Item = Framed<TcpStream, NTCodec>;
    type Error = ();

    fn poll(&mut self) -> Poll<Framed<TcpStream, NTCodec>, ()> {
        self.handle.poll()
    }
}

impl NetworkTables {
    pub fn connect(client_name: &'static str, target: &SocketAddr, state: Arc<Mutex<State>>) -> NetworkTables {
        state.lock().unwrap().set_state(ConnectionState::Connecting) ;
        let future = TcpStream::connect(target)
            .and_then(move |sock| {
                let codec = NTCodec.framed(sock);
                codec.send(Box::new(ClientHello::new(::NT_PROTOCOL_REV, client_name)))
            })
            .map_err(|_| ());
        NetworkTables {
            state,
            handle: Box::new(future),
        }
    }
}


pub fn poll(state: Arc<Mutex<State>>, codec: Framed<TcpStream, NTCodec>) -> impl Future<Item=(), Error=()> {
    let (tx, rx) = codec.split();
    rx
        .fold(tx, move |tx, packet| {
            let resp = match packet {
                Packet::ServerHello(packet) => {
                    println!("Got server hello: {:?}", packet);
                    Either::B(ok(tx))
                }
                Packet::ServerHelloComplete(_) => {
                    println!("Got server hello complete");
                    state.lock().unwrap().set_state(ConnectionState::Connected);
                    println!("Sent ClientHelloComplete");
                    Either::A(tx.send(Box::new(ClientHelloComplete)))
                }
                Packet::ProtocolVersionUnsupported(packet) => {
                    println!("Got this {:?}", packet);
                    Either::B(ok(tx))
                }
                Packet::EntryAssignment(entry /* heheheh */) => {
                    let mut state = state.lock().unwrap();
                    state.add_entry(entry.entry_id, entry.entry_value);

                    Either::B(ok(tx))
                }
                Packet::EntryDelete(delet) => {
                    let mut state = state.lock().unwrap();
                    state.remove_entry(delet.entry_id);

                    Either::B(ok(tx))
                }
                _ => Either::B(ok(tx))
            };

            resp
        })
        .map_err(|e| println!("Got error {:?}", e))
        .map(|_| ())
}

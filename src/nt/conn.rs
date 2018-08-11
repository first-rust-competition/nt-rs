use tokio_codec::Framed;
use tokio::net::TcpStream;
use futures::{Future, Poll, Stream, Sink, Async};
use futures::sync::mpsc::channel;
use tokio;
use tokio_core::reactor::Handle;
use tokio_codec::Decoder;

use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use proto::codec::NTCodec;
use proto::client::ClientHello;
use nt::state::{State, ConnectionState};
use nt::{send_packets, poll_socket};

pub struct Connection {
    future: Box<Future<Item=(), Error=()> + Send>,
}

impl Future for Connection {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.future.poll()
//        Ok(Async::NotReady)
    }
}

impl Connection {
    pub fn new(handle: &Handle, target: &SocketAddr, client_name: &'static str, state: Arc<Mutex<State>>) -> Connection {
        let handle = handle.clone().remote().clone();
        let end_state = state.clone();
        let err_state = state.clone();
        let future = TcpStream::connect(target)
            .and_then(move |sock| {
                let codec = NTCodec.framed(sock);
                codec.send(Box::new(ClientHello::new(::NT_PROTOCOL_REV, client_name)))
            })
            .map_err(move |e| {
                error!("{}", e);
                err_state.clone().lock().unwrap().set_connection_state(ConnectionState::Idle);
                debug!("{:?}", err_state.lock().unwrap().connection_state())
            })
            .and_then(move |codec| {
                info!("Connected, spawning tasks");
                let (tx, rx) = codec.split();
                let (chan_tx, chan_rx) = channel(5);

                handle.spawn(|_| send_packets(tx, chan_rx));
                poll_socket(state.clone(), rx, chan_tx.clone())
            })
            .then(move |_| {
                end_state.lock().unwrap().set_connection_state(ConnectionState::Idle);
                Ok(())
            });

        Connection {
            future: Box::new(future),
        }
    }
}
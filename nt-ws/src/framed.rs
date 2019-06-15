use nt_network::{ReceivedPacket, Packet, ClientHelloComplete};
use stdweb::web::{WebSocket, FileReader, FileReaderResult, IEventTarget, interval_buffered};
use bytes::BytesMut;
use std::rc::Rc;
use std::sync::Mutex;
use stdweb::web::event::{SocketMessageEvent, SocketMessageData, LoadEndEvent, IMessageEvent};
use nt_network::codec::NTCodec;
use tokio_codec::{Encoder, Decoder};
use futures::prelude::*;
use futures::channel::mpsc;


pub struct WsFramed {
    sock: WebSocket,
    rd: BytesMut,
    f: Box<dyn FnMut(&ReceivedPacket) + 'static>,
}

impl WsFramed {
    pub fn new(notify_tx: mpsc::UnboundedSender<()>, sock: WebSocket, listener: impl FnMut(&ReceivedPacket) + 'static) -> Rc<Mutex<WsFramed>> {
        let _self = Rc::new(Mutex::new(WsFramed { sock, rd: BytesMut::new(), f: Box::new(listener) }));

        let state = _self.clone();
        _self.lock().unwrap().sock.add_event_listener(move |event: SocketMessageEvent| {
            let notify_tx = notify_tx.clone();
            match event.data() {
                SocketMessageData::Blob(b) => {
                    let rdr = FileReader::new();
                    rdr.read_as_array_buffer(&b).unwrap();
                    let lrdr = rdr.clone();
                    let state = state.clone();
                    rdr.add_event_listener(move |_: LoadEndEvent| {
                        match lrdr.result().unwrap() {
                            FileReaderResult::ArrayBuffer(buf) => {
                                let mut state = state.lock().unwrap();
                                let v: Vec<u8> = Vec::from(buf);
                                state.rd.extend_from_slice(&v[..]);
                                while let Ok(Some(packet)) = NTCodec.decode(&mut state.rd) {
                                    let f = &mut state.f;
                                    f(&packet);
                                    if let ReceivedPacket::ServerHelloComplete = packet {
                                        state.send(Box::new(ClientHelloComplete));
                                        notify_tx.unbounded_send(()).unwrap();
                                        state.start_keepalive();
                                    }
                                }
                            }
                            _ => {} // Never read as stringk
                        }
                    });
                }
                _ => {}
            }
        });

        _self
    }

    pub fn socket(&self) -> &WebSocket {
        &self.sock
    }

    pub fn send(&self, packet: Box<dyn Packet>) {
        let mut wr = BytesMut::new();
        wr.reserve(1024); // reserve 1KiB
        NTCodec.encode(packet, &mut wr).expect("Encoder failed");
        self.sock.send_bytes(&wr.to_vec()[..]).expect("Send failed");
    }

    pub fn start_keepalive(&self) {
        let sock = self.sock.clone();
        stdweb::spawn_local(interval_buffered(1000)
            .for_each(move |_| {
                sock.send_bytes(&[0]).unwrap(); // Shorthand for KeepAlive packet
                futures::future::ready(())
            }));
    }
}
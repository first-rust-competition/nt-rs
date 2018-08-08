#![feature(attr_literals)]

extern crate futures;
extern crate tokio;
extern crate tokio_codec;
extern crate bytes;
extern crate nt_packet;
#[macro_use]
extern crate nt_packet_derive;
extern crate failure;

pub const NT_PROTOCOL_REV: u16 = 0x0300;

pub type Result<T> = std::result::Result<T, failure::Error>;

mod proto;

use proto::codec::NTCodec;
use proto::client::*;
use proto::Packet;

use tokio::net::TcpStream;
use tokio_codec::Decoder;
use futures::{Stream, Sink, Future};
use futures::future::ok;
use futures::future::Either;

fn main() -> Result<()> {
    let client = TcpStream::connect(&"127.0.0.1:1735".parse()?)
        .and_then(|sock| {
            let codec = NTCodec.framed(sock);
            let hello = ClientHello::new(NT_PROTOCOL_REV, "nt-rs");
            //TODO: State

            let start = codec.send(Box::new(hello)).and_then(|codec| {
                let (tx, rx) = codec.split();
                let poll = rx
                    .fold(tx, |tx, packet| {
                        let resp = match packet {
                            Packet::ServerHello(packet) => {
                                println!("Got server hello: {:?}", packet);
                                Either::B(ok(tx))
                            }
                            Packet::ServerHelloComplete(_) => {
                                println!("Got server hello complete");
                                println!("Sent ClientHelloComplete");
                                Either::A(tx.send(Box::new(ClientHelloComplete)))
                            }
                            Packet::ProtocolVersionUnsupported(packet) => {
                                println!("Got this {:?}", packet);
                                Either::B(ok(tx))
                            }
                            Packet::EntryAssignment(ass /* heheheh */) => {
                                println!("uuh {:?}", ass);
                                Either::B(ok(tx))
                            }
                            _ => Either::B(ok(tx))
                        };

                        resp
                    }).map_err(|e| println!("Got error {:?}", e))
                    .map(|_| ());

                tokio::spawn(poll);
                Ok(())
            }).then(|_| Ok(()));

            tokio::spawn(start);
            Ok(())
        })
        .map_err(|err| println!("Error = {:?}", err));

    tokio::run(client);

    Ok(())
}

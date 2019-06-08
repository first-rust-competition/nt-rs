use websocket::client::r#async::{Client, TcpStream};
use nt_network::codec::NTCodec;
use tokio::prelude::*;
use tokio::codec::{Encoder, Decoder};
use nt_network::{ReceivedPacket, Packet};
use futures::try_ready;
use websocket::OwnedMessage;
use bytes::BytesMut;
use failure::err_msg;

pub struct WSCodec {
    sock: Client<TcpStream>,
    codec: NTCodec,
}

impl WSCodec {
    pub fn new(sock: Client<TcpStream>) -> WSCodec {
        WSCodec { sock, codec: NTCodec }
    }
}

impl Stream for WSCodec {
    type Item = ReceivedPacket;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let msg: Option<OwnedMessage> = try_ready!(self.sock.poll());

        match msg {
            Some(OwnedMessage::Binary(bin)) => {
                let mut bytes = BytesMut::from(bin);
                match self.codec.decode(&mut bytes) {
                    Ok(Some(packet)) => Ok(Async::Ready(Some(packet))),
                    Ok(None) => Ok(Async::NotReady),
                    Err(e) => Err(e.into())
                }
            }
            Some(_) => Ok(Async::NotReady),
            None => Ok(Async::Ready(None)),
            _ => Ok(Async::NotReady)
        }
    }
}

impl Sink for WSCodec {
    type SinkItem = Box<dyn Packet>;
    type SinkError = failure::Error;

    fn start_send(&mut self, item: Self::SinkItem) -> Result<AsyncSink<Self::SinkItem>, Self::SinkError> {
        match self.poll_complete()? {
            Async::Ready(()) => {}
            Async::NotReady => return Ok(AsyncSink::NotReady(item))
        }

        let mut wr = BytesMut::new();
        self.codec.encode(item, &mut wr)?;
        match self.sock.start_send(OwnedMessage::Binary(wr.to_vec())) {
            Ok(AsyncSink::Ready) => Ok(AsyncSink::Ready),
            Err(e) => Err(e.into()),
            _ => Err(err_msg("Shouldn't be getting this"))
        }
    }

    fn poll_complete(&mut self) -> Result<Async<()>, Self::SinkError> {
        self.sock.poll_complete().map_err(|e| e.into())
    }

    fn close(&mut self) -> Result<Async<()>, Self::SinkError> {
        self.sock.close().map_err(|e| e.into())
    }
}
use tokio::net::TcpStream;
use tokio_util::codec::{Encoder, Decoder};
use nt_network::codec::NTCodec;
use futures_util::stream::Stream;
use futures_util::sink::Sink;
use futures_util::task::{Context, Poll};
use failure::_core::pin::Pin;
use nt_network::{ReceivedPacket, Packet};
use bytes::BytesMut;
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

pub struct WSCodec {
    sock: WebSocketStream<TcpStream>,
    rd: BytesMut
}

impl WSCodec {
    pub fn new(sock: WebSocketStream<TcpStream>) -> WSCodec {
        WSCodec {
            sock,
            rd: BytesMut::new(),
        }
    }
}

impl Stream for WSCodec {
    type Item = crate::Result<ReceivedPacket>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.rd.is_empty() {
            match futures_util::ready!(Stream::poll_next(Pin::new(&mut self.sock), cx)) {
                Some(msg) =>
                    match msg {
                        Ok(msg) => {
                            self.rd.extend_from_slice(&msg.into_data()[..]);
                            match NTCodec.decode(&mut self.rd) {
                                Ok(Some(packet)) => Poll::Ready(Some(Ok(packet))),
                                // Server should never split NT packets across multiple websocket packets
                                Ok(None) => panic!("We shouldn't get here nominally"),
                                Err(e) => Poll::Ready(Some(Err(e)))
                            }
                        }
                        Err(e) => Poll::Ready(Some(Err(e.into())))
                    }
                None => Poll::Ready(None)
            }
        }else {
            match NTCodec.decode(&mut self.rd) {
                Ok(Some(packet)) => Poll::Ready(Some(Ok(packet))),
                // Server should never split NT packets across multiple websocket packets
                Ok(None) => panic!("We shouldn't get here nominally"),
                Err(e) => Poll::Ready(Some(Err(e)))
            }
        }
    }
}

impl Sink<Box<dyn Packet>> for WSCodec {
    type Error = failure::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Sink::poll_ready(Pin::new(&mut self.sock), cx).map_err(Into::into)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Box<dyn Packet>) -> Result<(), Self::Error> {
        let mut wr = BytesMut::new();
        NTCodec.encode(item, &mut wr).unwrap();

        Sink::start_send(Pin::new(&mut self.sock), Message::Binary(wr.to_vec())).map_err(Into::into)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Sink::poll_flush(Pin::new(&mut self.sock), cx).map_err(Into::into)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Sink::poll_close(Pin::new(&mut self.sock), cx).map_err(Into::into)
    }
}

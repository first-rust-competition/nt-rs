use crate::error::{Error, Result};
use crate::proto::prelude::*;
use futures::{ready, Sink};
use std::task::Context;
use tokio::macros::support::{Pin, Poll};
use tokio::net::TcpStream;
use tokio::stream::Stream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

// TODO: MaybeTls adaptor
pub struct NTSocket(WebSocketStream<TcpStream>);

impl NTSocket {
    pub fn new(sock: WebSocketStream<TcpStream>) -> NTSocket {
        NTSocket(sock)
    }
}

impl Stream for NTSocket {
    type Item = Result<NTMessage>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(Stream::poll_next(Pin::new(&mut self.0), cx)) {
            Some(msg) => match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<Vec<NTTextMessage>>(&text) {
                        Ok(msgs) => Poll::Ready(Some(Ok(NTMessage::Text(msgs)))),
                        Err(e) => Poll::Ready(Some(Err(Error::JSON(e)))),
                    }
                }
                Ok(Message::Binary(bin)) => Poll::Ready(Some(Ok(NTMessage::Binary(
                    NTBinaryMessage::from_slice(&bin[..])
                        .into_iter()
                        .filter_map(|msg| msg.ok())
                        .collect(),
                )))),
                Ok(Message::Close(_)) => Poll::Ready(Some(Ok(NTMessage::Close))),
                Err(e) => Poll::Ready(Some(Err(Error::Tungstenite(e)))),
                _ => Poll::Pending,
            },
            None => Poll::Ready(None),
        }
    }
}

impl Sink<NTMessage> for NTSocket {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Sink::poll_ready(Pin::new(&mut self.0), cx).map_err(Into::into)
    }

    fn start_send(mut self: Pin<&mut Self>, item: NTMessage) -> Result<()> {
        match item {
            NTMessage::Text(msgs) => {
                let frame = Message::Text(serde_json::to_string(&msgs).unwrap());
                Sink::start_send(Pin::new(&mut self.0), frame).map_err(Into::into)
            }
            NTMessage::Binary(msgs) => {
                let body = msgs
                    .into_iter()
                    .flat_map(|msg| rmp_serde::to_vec(&msg).unwrap())
                    .collect::<Vec<u8>>();
                Sink::start_send(Pin::new(&mut self.0), Message::Binary(body)).map_err(Into::into)
            }
            NTMessage::Close => {
                Sink::start_send(Pin::new(&mut self.0), Message::Close(None)).map_err(Into::into)
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Sink::poll_flush(Pin::new(&mut self.0), cx).map_err(Into::into)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Sink::poll_close(Pin::new(&mut self.0), cx).map_err(Into::into)
    }
}

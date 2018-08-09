
/// Represents NT packet 0x01 Client Hello
/// Sent by a client to initialize a new connection to a server
#[derive(Debug, ClientMessage)]
#[packet_id = 0x01]
pub struct ClientHello {
    rev: u16,
    client_name: String,
}

impl ClientHello {
    pub fn new(rev: u16, name: &str) -> ClientHello {
        ClientHello {
            rev,
            client_name: name.to_string()
        }
    }
}

/// Represents NT packet 0x05 Client Hello Complete
/// Sent in response to 0x03 Server Hello Complete, in order to complete the handshake process
#[derive(Debug, ClientMessage)]
#[packet_id = 0x05]
pub struct ClientHelloComplete;
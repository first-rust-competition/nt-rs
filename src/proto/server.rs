
/// Represents NT packet 0x02 Protocol Version Unsupported
/// Sent by a server when a Client Hello packet (ID 0x01) contains an unsupported NT revision
#[derive(Debug, ServerMessage)]
#[packet_id = 0x02]
pub struct ProtocolVersionUnsupported {
    pub server_rev: u16,
}

/// Represents NT packet 0x04 Server Hello
/// Sent by a server in response to 0x01 Client Hello, when the connection has been accepted
#[derive(Debug, ServerMessage)]
#[packet_id = 0x04]
pub struct ServerHello {
    pub flags: u8,
    pub server_name: String,
}

/// Represents NT packet 0x03 Server Hello Complete
/// Sent by a server when the client has been updated with all of the entries of the server, and the connection handshake has been completed.
#[derive(Debug, ServerMessage)]
#[packet_id = 0x03]
pub struct ServerHelloComplete;


#[derive(Debug, ServerMessage)]
#[packet_id = 0x02]
pub struct ProtocolVersionUnsupported {
    server_rev: u16,
}

#[derive(Debug, ServerMessage)]
#[packet_id = 0x04]
pub struct ServerHello {
    flags: u8,
    server_name: String,
}

#[derive(Debug, ServerMessage)]
#[packet_id = 0x03]
pub struct ServerHelloComplete;

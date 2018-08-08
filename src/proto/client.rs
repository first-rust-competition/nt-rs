
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

#[derive(Debug, ClientMessage)]
#[packet_id = 0x05]
pub struct ClientHelloComplete;
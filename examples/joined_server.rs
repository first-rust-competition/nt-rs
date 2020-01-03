use nt::*;

#[tokio::main]
async fn main() {
    let mut nt = NetworkTables::bind_both("0.0.0.0:1735", "0.0.0.0:1835", "nt-rs");

    nt.add_connection_callback(ServerCallbackType::ClientConnected, |addr| {
        println!("Client connected: {}", addr);
    });

    nt.add_callback(CallbackType::Add, |data| {
        println!("Got new entry: {:?}", data);
    });

    loop {}
}
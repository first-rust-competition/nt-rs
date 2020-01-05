use nt::*;

#[tokio::main]
async fn main() {
    let mut nt = NetworkTables::bind("0.0.0.0:1735", "nt-rs-server");

    nt.add_connection_callback(ConnectionCallbackType::ClientConnected, |addr| {
        println!("Client connected! {}", addr);
    });
    nt.add_connection_callback(ConnectionCallbackType::ClientDisconnected, |addr| {
        println!("Client disconnected {}", addr);
    });

    nt.add_callback(CallbackType::Add, |data| {
        println!("Got new entry {:?}", data);
    });

    loop {}
}
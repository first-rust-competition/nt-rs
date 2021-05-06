#[tokio::main]
#[cfg(feature = "websocket")]
async fn main() {
    use nt::*;
    let mut nt = NetworkTables::connect_ws("ws://127.0.0.1:1735", "nt-ws")
        .await
        .unwrap();
    nt.add_callback(CallbackType::Add, |data| {
        println!("Got new entry {:?}", data)
    });

    nt.add_connection_callback(ConnectionCallbackType::ClientDisconnected, |_| {
        println!("Client disconnected");
    });
    println!("It connected!");
    let id = nt
        .create_entry(EntryData::new(
            "/foo".to_string(),
            0,
            EntryValue::String("bar".to_string()),
        ))
        .await
        .expect("Client is not connected to the server.");
    println!("Entry should have been created: {}", id);
    loop {}
}

#[tokio::main]
#[cfg(not(feature = "websocket"))]
async fn main() {
    panic!("Example needs the websocket feature enabled")
}

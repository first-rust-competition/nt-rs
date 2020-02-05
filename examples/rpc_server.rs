use nt::*;
use std::time::Duration;
use std::thread;

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

    nt.create_rpc(
        EntryData::new(
            "TEST_RPC".into(),
            0,
            EntryValue::RpcDefinition(RpcDefinition::V0),
        ),
        |parameter| {
            let mut sum = 0;
            for i in parameter.clone() {
                sum += i;
            }
            println!("{:?}", parameter);
            thread::sleep(Duration::from_millis(1000));
            vec![sum]
        },
    );

    loop {}
}

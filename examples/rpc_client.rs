use nt::*;
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let client = NetworkTables::connect("127.0.0.1:1735", "nt-rs").await?;

    loop {
        client
            .entries()
            .iter()
            .for_each(|(id, entry)| match entry.value {
                EntryValue::RpcDefinition(RpcDefinition::V0) => client.call_rpc(
                    *id,
                    vec![
                        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                    ],
                    |res| {
                        println!("RECEIVED RESPONSE: {:?}", res);
                    },
                ),
                _ => {}
            });
        thread::sleep(Duration::from_millis(1000));
    }
}

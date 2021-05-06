#[tokio::main]
#[cfg(feature = "websocket")]
async fn main() -> anyhow::Result<()> {
    use nt::*;
    use std::thread;
    use std::time::Duration;
    let nt = NetworkTables::connect_ws("ws://127.0.0.1:1735", "nt-ws").await?;

    let mut i = 0;
    loop {
        println!("RUNNING LOOP");
        nt.entries()
            .iter()
            .for_each(|(id, entry)| match entry.value {
                EntryValue::RpcDefinition(RpcDefinition::V0) => {
                    nt.call_rpc(*id, (0..(i % 20)).collect(), |res| {
                        println!("RECEIVED RESPONSE: {:?}", res);
                    })
                }
                _ => {}
            });
        thread::sleep(Duration::from_millis(100));
        i += 1;
    }
}

#[tokio::main]
#[cfg(not(feature = "websocket"))]
async fn main() -> anyhow::Result<()> {
    panic!("Example needs the websocket feature enabled")
}

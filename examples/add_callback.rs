use nt::{CallbackType, NetworkTables};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = NetworkTables::connect("127.0.0.1:1735", "nt-rs").await?;

    client.add_callback(CallbackType::Add, |new_entry| {
        println!("A new entry was received! {:?}", new_entry);
    });

    client.add_callback(CallbackType::Delete, |deleted_entry| {
        println!("An entry was deleted. {:?}", deleted_entry);
    });

    client.add_callback(CallbackType::Update, |updated_entry| {
        println!("An entry was updated. New value: {:?}", updated_entry)
    });

    Ok(())
}

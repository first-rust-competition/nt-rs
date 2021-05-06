use nt::{EntryData, EntryValue, NetworkTables};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = NetworkTables::connect("127.0.0.1:1735", "nt-rs").await?;

    let _id = client
        .create_entry(EntryData::new(
            "newEntry".to_string(),
            0,
            EntryValue::Double(5.0),
        ))
        .await?;

    for (id, value) in client.entries() {
        println!("{} ==> {:?}", id, value);
    }

    Ok(())
}

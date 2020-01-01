use nt::{NetworkTables, EntryData, EntryValue};

type Result<T> = std::result::Result<T, failure::Error>;

#[tokio::main]
async fn main() -> Result<()> {
    let client = NetworkTables::connect("127.0.0.1:1735", "nt-rs").await?;

    client.create_entry(EntryData::new("newEntry".to_string(), 0, EntryValue::Double(5.0)));

    for (id, value) in client.entries() {
        println!("{} ==> {:?}", id, value);
    }

    Ok(())
}

use nt::{EntryData, EntryValue, NetworkTables};
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //    setup_logger()?;

    let client = NetworkTables::connect("127.0.0.1:1735", "nt-rs").await?;
    println!("Creating entry");
    let entry_id = client
        .create_entry(EntryData::new(
            "update1".to_string(),
            0,
            EntryValue::String("Hello!".to_string()),
        ))
        .await?;
    println!("Entry created");

    {
        let mut entry = client.get_entry(entry_id);
        println!("{} ==> {:?}", entry.id(), entry.value());

        println!("Changing value");
        entry.set_value(EntryValue::String("World!".to_string()));
        println!("2 {} ==> {:?}", entry.id(), entry.value());
    }

    thread::sleep(Duration::from_millis(250));

    Ok(())
}

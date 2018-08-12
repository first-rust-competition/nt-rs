extern crate nt;
extern crate failure;
extern crate fern;
extern crate log;
extern crate chrono;

type Result<T> = std::result::Result<T, failure::Error>;

use nt::{NetworkTables, EntryData, EntryValue};


fn main() -> Result<()> {

    let mut client = NetworkTables::connect("nt-rs", "127.0.0.1:1735".parse()?);

    client.create_entry(EntryData::new("update1".to_string(), 0, EntryValue::String("Hello!".to_string())));

    let (id, entry) = client.entries().into_iter().find(|(_, v)| v.name == "update1".to_string()).unwrap();

    println!("{} ==> {:?}", id, entry);

    client.update_entry(id, EntryValue::String("World!".to_string()));

    for (id, entry) in client.entries() {
        println!("{} ==> {:?}", id, entry);
    }

    Ok(())
}
fn setup_logger() -> Result<()> {
    fern::Dispatch::new()
        .format(|out, msg, record| {
            out.finish(format_args!(
                "{} [{}] [{}] {}",
                chrono::Local::now().format("[%Y-%m-%d] [%H:%M:%S]"),
                record.target(),
                record.level(),
                msg
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

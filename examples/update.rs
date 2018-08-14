#![feature(nll)]

extern crate nt;
extern crate failure;
extern crate fern;
extern crate log;
extern crate chrono;

type Result<T> = std::result::Result<T, failure::Error>;

use nt::{NetworkTables, EntryData, EntryValue};


fn main() -> Result<()> {
    setup_logger()?;

    let mut client = NetworkTables::connect("nt-rs", "127.0.0.1:1735".parse()?)?;
    let entry_id = client.create_entry(EntryData::new("update1".to_string(), 0, EntryValue::String("Hello!".to_string())));

    {
        let mut entry = client.get_entry_mut(entry_id);
        println!("{} ==> {:?}", entry.id(), entry.value());

        entry.set_value(EntryValue::String("World!".to_string()));
        println!("{} ==> {:?}", entry.id(), entry.value());
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
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

extern crate nt;
extern crate failure;
extern crate fern;
#[macro_use]
extern crate log;
extern crate chrono;

use nt::{NetworkTables, EntryData, EntryValue};

type Result<T> = std::result::Result<T, failure::Error>;

fn main() -> Result<()> {
    setup_logger()?;
    let mut client = NetworkTables::connect("nt-rs", "127.0.0.2:1735".parse()?)?;

    client.create_entry(EntryData::new("newEntry".to_string(), 0, EntryValue::Double(5.0)));

    for (id, value) in client.entries() {
        println!("{} ==> {:?}", id, value);
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

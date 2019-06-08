extern crate nt;
extern crate fern;
extern crate log;
extern crate failure;
extern crate chrono;

use nt::NetworkTables;
use nt::CallbackType;

type Result<T> = std::result::Result<T, failure::Error>;

fn main() -> Result<()> {
    setup_logger()?;

    let mut client = NetworkTables::connect("127.0.0.1:1735", "nt-rs")?;
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

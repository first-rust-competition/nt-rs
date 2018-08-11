extern crate nt;
extern crate tokio;
extern crate tokio_codec;
extern crate futures;
extern crate fern;
#[macro_use]
extern crate log;
extern crate failure;
extern crate chrono;

type Result<T> = std::result::Result<T, failure::Error>;

use nt::NetworkTables;

use std::io;

mod input;
use input::Message;

fn main() -> Result<()> {
    setup_logger()?;

    let mut client = NetworkTables::connect("nt-test", "127.0.0.1:1735".parse()?);

    while client.connected() {

        let mut s = String::new();
        io::stdin().read_line(&mut s);

        if let Some(msg) = input::parse_message(s) {
            match msg {
                Message::Add(dat) => client.create_entry(dat),
            }
        }
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

use nt::*;
use chrono::Local;
use log::LevelFilter;
use std::io;
use fern::colors::{ColoredLevelConfig, Color};
use tokio::time::Duration;

fn init_logger() {
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .error(Color::Red)
        .warn(Color::Yellow)
        .debug(Color::White);
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{} {} {}  {}",
                Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                colors.color(record.level()),
                record.target(),
                message
            ))
        })
        .level(LevelFilter::Info)
        .chain(io::stdout())
        .apply()
        .unwrap();
}

#[tokio::main]
async fn main() {
    init_logger();
    let mut _nt = NetworkTables::bind("0.0.0.0:5810", nt::system_time).await;

    //TODO: content
    loop {}
}

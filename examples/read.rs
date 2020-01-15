extern crate failure;
extern crate nt;

type Result<T> = std::result::Result<T, failure::Error>;

use nt::{ConnectionCallbackType, NetworkTables};

#[tokio::main]
async fn main() -> Result<()> {
    let client = NetworkTables::connect("127.0.0.1:1735", "cool client").await?;

    client.add_connection_callback(ConnectionCallbackType::ClientDisconnected, |_| {
        println!("Client has disconnected from the server");
    });

    println!("Listing entries");
    for (id, data) in client.entries() {
        println!("{} => {:?}", id, data);
    }

    loop {}
    Ok(())
}

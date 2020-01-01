extern crate nt;
extern crate failure;

type Result<T> = std::result::Result<T, failure::Error>;

use nt::NetworkTables;

#[tokio::main]
async fn main() -> Result<()> {

    let client = NetworkTables::connect("127.0.0.1:1735", "cool client").await?;

    println!("Listing entries");
    for (id, data) in client.entries() {
        println!("{} => {:?}", id, data);
    }

    loop{}
    Ok(())
}


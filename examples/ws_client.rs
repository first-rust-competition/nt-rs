use nt::*;

fn main() {
    let mut nt = NetworkTables::connect_ws("ws://10.40.69.2:1835", "nt-ws").unwrap();
    //let mut nt = NetworkTables::connect_ws("ws://127.0.0.1:1835", "nt-ws").unwrap();
    nt.add_callback(CallbackType::Add, |data| {
        println!("Got new entry {:?}", data)
    });
    println!("It connected!");
    nt.create_entry(EntryData::new("/foo".to_string(), 0, EntryValue::String("bar".to_string())));
    println!("Entry should have been created");
    loop {}
}

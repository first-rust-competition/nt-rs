#[macro_use]
extern crate stdweb;

use nt_ws::*;
use nt_ws::entry::EntryData;
use nt_network::types::EntryValue;
use futures::prelude::*;

fn main() {
    stdweb::initialize();
    stdweb::spawn_local(NetworkTables::connect("ws://127.0.0.1:1735", "nt-ws")
        .then(|nt| {
            stdweb::spawn_local(nt.create_entry(EntryData::new("/foo".to_string(), 0, EntryValue::Double(1f64)))
                .then(move |id| {
                    for (id, data) in nt.entries().iter() {
                        js! { @(no_return)
                            console.log(@{format!("{} => {:?}", id, data)});
                        }
                    }
                    futures::future::ready(())
                }));


            futures::future::ready(())
        }));

    stdweb::event_loop();
}
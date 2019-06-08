# nt-ws
nt-ws provides a wasm-compatible NetworkTables client that can communicate over websockets with a compatible server.

This library is compatible with both client side Rust compiled with the `wasm32-unknown-unknown` target, and clientside JavaScript by means of wasm-bindgen.

## Example usage: Rust
```rust
use nt_ws::*;

fn main() {
    stdweb::initialize();
    
    stdweb::spawn_local(NetworkTables::connect("ws://10.TE.AM.2:1735", "nt-ws").then(|nt| {
        stdweb::spawn_local(nt.create_entry(EntryData::new("/foo".to_string(), 0, EntryValue::Double(1f64))).then(move |id| {
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
```

## Example usage: JavaScript
```javascript
import("nt-client").then(ntws => {
    ntws.NetworkTables.connect("ws://127.0.0.1:1735", "nt-js").then(nt => {
        nt.create_entry(new ntws.EntryData("/foo", 0, ntws.EntryType.String, "bar")).then(id => {
            console.log("Created entry with id " + id);
        });
    });
});
```

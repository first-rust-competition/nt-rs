# nt-rs
nt-rs is an implementation of the NetworkTables revision 3 protocol in Rust using Tokio.
Currently it is purely a client library. An API for creation of an NT server may come in the future.

# Getting Started
This library can be found on https://crates.io and can be added to a project with `nt = "0.1.0"`
Sample usages can be found in the `examples/` directory

# What is NetworkTables?
NetworkTables, or NT, is a TCP based protocol for key-value data storage. It is most used within code for _FIRST_ Robotics Competition robots, for sharing data across the robot's network.

# Examples
### Connecting to a server
```rust
let mut nt = NetworkTables::connect("10.TE.AM.2:1735", "nt-rs-client")?;
```
### Creating a server
```rust
let mut nt = NetworkTables::bind("0.0.0.0:1735", "nt-rs-server");
```

## Websockets
`nt` 1.0.0 adds support for clients and servers communicating over websockets. This is locked behind the `websockets` feature.
### Connecting to a websocket server
```rust
let mut nt = NetworkTables::connect_ws("ws://10.TE.AM.2:1735", "nt-ws-client")?;
```

### Creating a websocket server
```rust
let mut nt = NetworkTables::bind_ws("0.0.0.0:1735", "nt-ws-server");
```

## Network Tables in JavaScript (WIP)
While `nt` cannot be compiled to wasm directly, due to its dependence on tokio for its runtime, a supplementary crate is provided at `./nt-ws` that is able to be compiled to wasm. This crate only supports a NT client, and uses wasm_bindgen to export a javascript compatible API.

# License
This project is licensed under the MIT license.

# Acknowledgements
Thanks to Jess Creighton (https://github.com/jcreigh) for giving me the idea
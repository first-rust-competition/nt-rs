# nt-rs
nt-rs is an implementation of the NetworkTables revision 3 protocol in Rust using Tokio.
Currently it is purely a client library. An API for creation of an NT server may come in the future.

# Getting Started
This library can be found on https://crates.io and can be added to a project with `nt = "0.1.0"`
Sample usages can be found in the `examples/` directory

# What is NetworkTables?
NetworkTables, or NT, is a TCP based protocol for key-value data storage. It is most used within code for _FIRST_ Robotics Competition robots, for sharing data across the robot's network.

# Checklist 
- [x] RPC support (Fixed in nt-rs 0.3.0)

# License
This project is licensed under the MIT license.

# Acknowledgements
Thanks to Jess Creighton (https://github.com/jcreigh) for giving me the idea
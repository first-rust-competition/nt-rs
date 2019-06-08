List of things that should get done before publishing a new version with this code

[x] `close_rx` is thrown away in the construction of any server, and hence the connection could persist after the NetworkTables handle to the state is discarded. Since all States are Arcs, this will also leak that memory.
[ ] The WS Client in `nt` uses the websockets crate, and is impossible to use in the browser. Reconsider whether it needs to be an option
[x] Rewrite documentation
[x] KeepAlive isn't sent by the clients at all
[ ] Neither client nor server supports any version of RPC in the v3 spec. Determine whether this is an important usecase for nt-rs
[x] Timeout currently does nothing to prevent dead connections on the server side
[x] Block `NetworkTables<ClientState>` ctors until the connection is established
[x] Timeout on the blocking channels to abort a dead connection, return Result for clients in this case


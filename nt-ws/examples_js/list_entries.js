import * as nt from "nt-ws";

nt.NetworkTables.connect("ws://127.0.0.1:1735", "nt-js").then(inst => {
    for(var [key, value] of inst.entries()) {
        console.log(key + " => " + value);
    }
});

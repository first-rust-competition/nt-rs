import * as nt from "nt-ws";

nt.NetworkTables.connect("ws://127.0.0.1:1735", "nt-js").then(inst => {
    inst.create_entry(new nt.EntryData("/foo", 0, nt.EntryType.Double, 1.0)).then(id => {
        console.log("Created entry with id " + id);
    });
});

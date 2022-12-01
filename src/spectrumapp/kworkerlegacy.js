importScripts("HEREwbgpath");
// let modpath = "HEREwbgpath";
// import { default as init, child_entry_point } from modpath;
console.log("kworker.js importScripts...");
console.log("onmessage_b", wasmbindmod);
(async () => {
    donewbgmod = self;
    await donewbgmod.default(...event.data);

    for (let i = 0; i < preinitqueue.length; i++) {
        donewbgmod.child_entry_point(preinitqueue[i]);
    }
    preinitqueue = [];
})()
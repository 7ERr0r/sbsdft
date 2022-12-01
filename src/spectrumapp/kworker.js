





let firstMsg = true;
let donewbgmod = null;
let preinitqueue = [];
let wasmbindmodpromise = null;

self.addEventListener('message', event => {
    if (donewbgmod != null) {
        // fastpath
        donewbgmod.child_entry_point(event.data);
    } else {
        if (firstMsg) {
            firstMsg = false;

            (async () => {
                // let xd = await import("https://kloxki.com/dist_wgpu/alloc_hooks.js?3");
                // for (var prop in xd) {
                //     if (Object.prototype.hasOwnProperty.call(obj, prop)) {
                //         self[prop] = xd[prop];
                //     }
                // }


                let wasmbindmod = await wasmbindmodpromise;

                console.log("onmessage_a", wasmbindmod);
                try {
                    await wasmbindmod.default(...event.data);
                    donewbgmod = wasmbindmod;

                    for (let i = 0; i < preinitqueue.length; i++) {
                        donewbgmod.child_entry_point(preinitqueue[i]);
                    }
                    preinitqueue = [];
                } catch (err) {
                    setTimeout(() => {
                        throw err;
                    });
                }
            })()
        } else {
            preinitqueue.push(event.data);
        }
    }
});
wasmbindmodpromise = import(wbgpath);







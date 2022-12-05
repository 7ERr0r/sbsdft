





let firstMsg = true;
let wasmbinds = null;
let preinitqueue = [];
let importpromise = null;

self.addEventListener('message', event => {
    if (wasmbinds != null) {
        // fastpath
        wasmbinds.child_entry_point(event.data);
    } else {
        if (firstMsg) {
            firstMsg = false;

            (async () => {
                let wasmbindmod = await importpromise;

                console.log("kworker.js first message", wasmbindmod);
                try {
                    await wasmbindmod.default(...event.data);
                    wasmbinds = wasmbindmod;

                    for (let i = 0; i < preinitqueue.length; i++) {
                        wasmbinds.child_entry_point(preinitqueue[i]);
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
importpromise = import(wbgpath);







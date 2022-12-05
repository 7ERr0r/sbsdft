importScripts("replace_with_bindgen_js");

// console.log("kworkerlegacy.js importScripts done");

// console.log("kworkerlegacy.js: ", wasm_bindgen);
let firstMsg = true;
self.addEventListener('message', event => {
    if (firstMsg) {
        firstMsg = false;
        console.log("kworkerlegacy.js first msg");
        wasmbinds = wasm_bindgen;
        wasm_bindgen.initSync(...event.data);
    } else {
        wasmbinds.child_entry_point(event.data);
    }
})
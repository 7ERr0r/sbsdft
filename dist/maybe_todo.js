const root_global = (() => eval)()('this');
if (root_global.bindgen_export_without_modules === undefined) {
    root_global.bindgen_export_without_modules = wasm_bindgen;
}
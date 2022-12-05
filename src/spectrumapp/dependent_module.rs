use js_sys::{Array, ArrayBuffer, JsString, Uint8Array};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, Request, RequestInit, RequestMode, Response, Url};

use super::pool::{get_wasm_bindgen_js_path, get_wasm_bindgen_using_modules};

// // This is a not-so-clean approach to get the current bindgen ES module URL
// // in Rust. This will fail at run time on bindgen targets not using ES modules.
// #[wasm_bindgen]
// extern "C" {
//     #[wasm_bindgen]
//     type ImportMeta;

//     #[wasm_bindgen(method, getter)]
//     fn url(this: &ImportMeta) -> JsString;

//     #[wasm_bindgen(js_namespace = import, js_name = meta)]
//     static IMPORT_META: ImportMeta;
//     // #[wasm_bindgen(js_name = import)]
//     // static IMPORT: JsValue;
// }

// pub fn get_import_meta() -> Option<String> {
//     // let global = js_sys::global();
//     // klog!("global: {:?}", global);
//     // let import = js_sys::Reflect::get(&IMPORT, &JsString::from("import")).ok()?;
//     // klog!("import: {:?}", import);

//     let meta =
//         js_sys::eval("import.meta").unwrap();
//     //let meta = js_sys::Reflect::get(&IMPORT, &JsString::from("meta")).ok()?;
//     klog!("meta: {:?}", meta);
//     let meta: ImportMeta = meta.dyn_into().ok()?;
//     Some(meta.url().into())
// }

pub async fn fetch_js_file(url: &str) -> Result<ArrayBuffer, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = web_sys::window().ok_or_else(|| JsString::from("window not found"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    // `resp_value` is a `Response` object.
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into()?;

    // Convert this other `Promise` into a rust `Future`.
    let array = JsFuture::from(resp.array_buffer()?).await?;
    assert!(array.is_instance_of::<ArrayBuffer>());
    let array: ArrayBuffer = array.dyn_into()?;
    Ok(array)
}

#[allow(unused)]
pub async fn on_the_fly(code: &str) -> Result<String, JsValue> {
    // Generate the import of the bindgen ES module, assuming `--target web`:

    let window = web_sys::window().unwrap();

    let nav = window.navigator();

    //let using_modules = nav.user_agent()?.to_lowercase().contains("firefox");
    let using_modules = get_wasm_bindgen_using_modules();
    let header;
    if using_modules {
        header = format!(
            "import init, * as bindgen from '{}';\nconst wasmbinds = bindgen;\n\n",
            get_wasm_bindgen_js_path().unwrap_or("None"),
        );
    } else {
        // importScripts does not work
        // await import() does not work

        // header = format!(
        //     "importScripts(\"{}\");\nconst bindgen = wasm_bindgen;\n",
        //     get_wasm_bindgen_js_path().unwrap_or("None"),
        // );
        // header = format!(
        //     "let xx = await import('{}');\n\n",
        //     get_wasm_bindgen_js_path().unwrap_or("None"),
        // );
        let bindgen_script = fetch_js_file(get_wasm_bindgen_js_path().unwrap()).await?;
        let bindgen_script = Uint8Array::new(&bindgen_script);
        let script = bindgen_script.to_vec();
        let script = String::from_utf8_lossy(&script);
        header = format!("{}\nconst wasmbinds = wasm_bindgen;\n\n", script,);
    }

    Url::create_object_url_with_blob(&Blob::new_with_str_sequence_and_options(
        &Array::of2(&JsValue::from(header.as_str()), &JsValue::from(code)),
        &BlobPropertyBag::new().type_("text/javascript"),
    )?)
}

// // dependent_module! takes a local file name to a JS module as input and
// // returns a URL to a slightly modified module in run time. This modified module
// // has an additional import statement in the header that imports the current
// // bindgen JS module under the `bindgen` alias, and the separate init function.
// // How this URL is produced does not matter for the macro user. on_the_fly
// // creates a blob URL in run time. A better, more sophisticated solution
// // would add wasm_bindgen support to put such a module in pkg/ during build time
// // and return a URL to this file instead (described in #3019).
// #[macro_export]
// macro_rules! dependent_module {
//     ($file_name:expr) => {
//         crate::spectrumapp::dependent_module::on_the_fly(include_str!($file_name))
//     };
// }

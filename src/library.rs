use std::mem::MaybeUninit;

// #[cfg(target_arch = "wasm32")]
// use dlmalloc::GlobalDlmalloc;

// #[cfg(target_arch = "wasm32")]
// #[global_allocator]
// static GLOBAL: GlobalDlmalloc = GlobalDlmalloc;

// extern crate wee_alloc;

// // Use `wee_alloc` as the global allocator.
// #[global_allocator]
// static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// use crate::spectrumapp::tracingalloc::KWasmTracingAllocator;
// #[cfg(target_arch = "wasm32")]
// #[global_allocator]
// static GLOBAL: KWasmTracingAllocator<std::alloc::System> =
//     KWasmTracingAllocator(std::alloc::System);

pub mod counteralloc;

// #[global_allocator]
// static GLOBAL: counteralloc::Counter = counteralloc::Counter;

#[cfg(target_arch = "wasm32")]
pub mod spectrumapp;

// #[cfg(target_arch = "wasm32")]
// #[wasm_bindgen(start)]
// pub fn start() {
//     // do nothing (threads also call this)
// }
#[cfg(target_arch = "wasm32")]
pub fn main() {
    // do nothing (threads also call this)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start_dft(wasm_bindgen_path: &str) {
    use wasm_bindgen::prelude::*;

    std::alloc::set_alloc_error_hook(|layout| {
        panic!("memory allocation of {} bytes failed", layout.size());
    });
    std::panic::set_hook(Box::new(|info| {
        unsafe {
            crate::spectrumapp::PANICKED = true;
        }
        use crate::klog;
        klog!("panicking...");

        console_error_panic_hook::hook(info);

        klog!(
            "mem size: {}",
            crate::spectrumapp::kwasm::get_wasm_mem_size()
        );
        //klog!("rust panic: {}", info);
    }));

    use crate::spectrumapp::kwasm::prealloc_fast;
    let v = prealloc_fast(200 * 1024 * 1024);
    crate::spectrumapp::kwasm::debug_wasm_mem("start_dft");
    use crate::spectrumapp::wasm_rayon::init_wasm_rayon;
    unsafe {
        crate::spectrumapp::pool::set_wasm_bindgen_js_path(wasm_bindgen_path);
    }
    init_wasm_rayon();
    crate::spectrumapp::kwasm::debug_wasm_mem("init_wasm_rayon");

    drop(v);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start_spectrumapp() {
    crate::spectrumapp::kwasm::debug_wasm_mem("start_spectrumapp");
    spectrumapp::main();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn main() {}

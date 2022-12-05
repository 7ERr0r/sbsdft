#![cfg_attr(target_arch = "wasm32", feature(alloc_error_hook))]

// #[cfg(target_arch = "wasm32")]
// pub use wasm_bindgen_rayon::init_thread_pool;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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

pub(crate) mod counteralloc;

// #[global_allocator]
// static GLOBAL: counteralloc::Counter = counteralloc::Counter;

#[cfg(target_arch = "wasm32")]
pub(crate) mod spectrumapp;

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
pub fn start_dft(wasm_bindgen_path: &str, rayon_threads: i32) {
    use crate::spectrumapp::appstate::*;
    set_app_state(AppState::Initializing);

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

    set_app_state(AppState::InitializingMem);

    use crate::spectrumapp::kwasm::prealloc_fast;
    let v = prealloc_fast(200 * 1024 * 1024);
    crate::spectrumapp::kwasm::debug_wasm_mem("start_dft");

    unsafe {
        crate::spectrumapp::pool::set_wasm_bindgen_js_path(wasm_bindgen_path);
    }

    let meta = spectrumapp::dependent_module::get_import_meta();
    klog!("meta: {:?}", meta);

    set_app_state(AppState::InitRayon);
    use crate::spectrumapp::wasm_rayon::init_wasm_rayon;

    init_wasm_rayon(rayon_threads.max(2).min(8) as u32);

    crate::spectrumapp::kwasm::debug_wasm_mem("init_wasm_rayon");

    drop(v);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start_spectrumapp(is_mobile: bool) {
    crate::spectrumapp::kwasm::debug_wasm_mem("start_spectrumapp");
    spectrumapp::main(is_mobile);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn main() {}

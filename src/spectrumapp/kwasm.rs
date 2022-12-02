#[cfg(target_arch = "wasm32")]
use crate::klog;

#[cfg(not(target_arch = "wasm32"))]
pub fn fix_webgl_color(wgsl_shader: &str) -> String {
    String::from(wgsl_shader)
}

#[cfg(target_arch = "wasm32")]
pub fn fix_webgl_color(wgsl_shader: &str) -> String {
    // TODO check if still necessary
    //let wgsl_shader = wgsl_shader.replace("// webglcolorfix", "color = color / 128.0;");
    //let wgsl_shader = wgsl_shader.replace("// webgltexfix", "color = color / 200.0;");

    wgsl_shader.into()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_once<F, T>(_tname: &'static str, f: F) -> ()
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(f);
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_once<F, T>(tname: &'static str, f: F) -> ()
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    let js_pool = super::pool::WorkerPool::new(1).unwrap();
    let js_worker = js_pool.worker().unwrap();

    let js_workerc = js_worker.clone();
    klog!("spawn_once...");
    super::pool::execute_unpooled(&js_workerc, move || {
        klog!("spawn_once execute_unpooled {}", tname);
        f();
    })
    .unwrap();

    Box::leak(Box::new(js_pool));
    Box::leak(Box::new(js_worker));
}

#[cfg(target_arch = "wasm32")]
pub fn get_wasm_mem_size() -> i64 {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;
    let mem = wasm_bindgen::memory();

    let mem_buffer = Reflect::get(&mem, &JsValue::from("buffer")).unwrap();
    let byte_length = Reflect::get(&mem_buffer, &JsValue::from("byteLength")).unwrap();
    let byte_length = byte_length.as_f64().unwrap() as i64;

    byte_length
}
#[cfg(not(target_arch = "wasm32"))]
pub fn get_wasm_mem_size() -> i64 {
    0
}

#[cfg(target_arch = "wasm32")]
pub fn debug_wasm_mem(name: &str) {
    klog!(
        "{}, mem: {:.2} MB",
        name,
        get_wasm_mem_size() as f64 / (1024.0 * 1024.0)
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub fn debug_wasm_mem(_name: &str) {}

pub fn prealloc_fast(len: usize) -> Vec<Vec<u8>> {
    let mut sum: u32 = 0;
    let mut vecs = Vec::new();
    const DIVISIONS: usize = 16;
    for _ in 0..DIVISIONS {
        let capacity = len / DIVISIONS;
        crate::spectrumapp::kwasm::debug_wasm_mem(&format!("prealloc_fast {}", capacity));
        let mut v: Vec<u8> = Vec::with_capacity(capacity);

        unsafe {
            v.set_len(v.capacity());
            let begin = v.as_mut_ptr();
            let end = begin.add(v.capacity());

            let mut iptr = begin;
            while iptr < end {
                iptr = iptr.add(64 * 1024);
                *iptr = 42;
                sum += *(iptr.add(1)) as u32;
            }
        }
        vecs.push(v);
    }
    black_box(sum);
    vecs
}

pub fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = std::ptr::read_volatile(&dummy);
        std::mem::forget(dummy);
        ret
    }
}

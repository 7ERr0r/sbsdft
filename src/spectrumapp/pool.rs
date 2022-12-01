// Silences warnings from the compiler about Work.func and child_entry_point
// being unused when the target is not wasm.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

//! A small module that's intended to provide an example of creating a pool of
//! web workers which can be used to execute `rayon`-style work.

use crate::klog;

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, Url};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent, WorkerOptions};
use web_sys::{ErrorEvent, Event, Worker};

#[wasm_bindgen]
#[derive(Clone)]
pub struct WorkerPool {
    state: Rc<PoolState>,
}

struct PoolState {
    workers: RefCell<Vec<Worker>>,
    callback: Closure<dyn FnMut(Event)>,
}

struct Work {
    func: Box<dyn FnOnce() + Send>,
}

pub static mut WASM_BINDGEN_JS_PATH: Option<String> = None;
pub unsafe fn set_wasm_bindgen_js_path(js_path: &str) {
    if WASM_BINDGEN_JS_PATH.is_none() {
        WASM_BINDGEN_JS_PATH = Some(String::from(js_path));
    }
}
pub fn get_wasm_bindgen_js_path() -> Option<&'static str> {
    unsafe { WASM_BINDGEN_JS_PATH.as_ref().map(|s| s.as_str()) }
}

/// Unconditionally spawns a new worker
///
/// The worker isn't registered with this `WorkerPool` but is capable of
/// executing work for this wasm module.
///
/// # Errors
///
/// Returns any error that may happen while a JS web worker is created and a
/// message is sent to it.
pub fn spawn_worker() -> Result<Worker, JsValue> {
    //const NAME: &'static str = env!("CARGO_PKG_NAME");
    //let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    //let location = global.location();
    //let origin = location.origin();
    //let pathname = location.pathname();

    //let path_split: Vec<&str> = pathname.rsplitn(2, '/').collect();
    //let root = path_split[1];
    let wrapper_url =
        get_wasm_bindgen_js_path().ok_or_else(|| JsValue::from("WASM_BINDGEN_JS_PATH not set"))?;
    //let wrapper_url = format!("{}{}/{}.js", origin, root, NAME);

    //     let worker_content = format!(r#"
    //     console.log('worker: importing');
    //     importScripts("{}");
    //     console.log('worker: setting onmessage');
    //     self.onmessage=(a=>{{
    //         let e = wasm_bindgen(...a.data).catch(a=>{{
    //             throw setTimeout(()=>{{throw a}}),a
    //         }});
    //         self.onmessage=(async a=>{{
    //             await e;
    //             wasm_bindgen.child_entry_point(a.data);
    //         }});
    //     }});
    // "#, wrapper_url);

    let mut worker_content = String::new();

    let window = web_sys::window().unwrap();

    let nav = window.navigator();
    let firefox = nav.user_agent()?.to_lowercase().contains("firefox");
    use std::fmt::Write;
    if firefox {
        worker_content = include_str!("kworkerlegacy.js").replace("HEREwbgpath", wrapper_url);
    } else {
        let _ = write!(
            worker_content,
            "const wbgpath = '{}';\n{}",
            wrapper_url,
            include_str!("kworker.js")
        );
    }
    klog!("creating worker (len: {})", worker_content.len());

    // klog!(
    //     "creating worker (len: {}): {}",
    //     worker_content.len(),
    //     worker_content
    // );
    let mut worker_blob_property_bag = BlobPropertyBag::new();
    worker_blob_property_bag.type_("text/javascript");

    // alloc new and copy directly from wasm memory
    let content_js_array =
        js_sys::Uint8Array::new(unsafe { &js_sys::Uint8Array::view(worker_content.as_bytes()) });

    let blob_parts = js_sys::Array::new();
    blob_parts.push(&content_js_array);

    let worker_blob =
        Blob::new_with_str_sequence_and_options(&blob_parts, &worker_blob_property_bag)?;
    let worker_data_url: String = Url::create_object_url_with_blob(&worker_blob)?;
    let mut worker_opts = WorkerOptions::new();
    worker_opts.name("kxworker");
    js_sys::Reflect::set(
        &worker_opts,
        &JsValue::from("type"),
        &JsValue::from("module"),
    )?;

    js_sys::Reflect::set(
        &worker_opts,
        &JsValue::from("credentials"),
        &JsValue::from("same-origin"),
    )?;
    let worker = Worker::new_with_options(&worker_data_url, &worker_opts)?;

    // With a worker spun up send it the module/memory so it can start
    // instantiating the wasm module. Later it might receive further
    // messages about code to run on the wasm module.
    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::module());
    array.push(&wasm_bindgen::memory());
    worker.post_message(&array)?;

    Ok(worker)
}

pub fn execute_unpooled(
    worker: &Worker,
    sent_func: impl FnOnce() + Send + 'static,
) -> Result<Worker, JsValue> {
    let work = Box::new(Work {
        func: Box::new(sent_func),
    });
    let ptr = Box::into_raw(work);
    match worker.post_message(&JsValue::from(ptr as u32)) {
        Ok(()) => Ok(worker.clone()),
        Err(e) => {
            unsafe {
                drop(Box::from_raw(ptr));
            }
            Err(e)
        }
    }
}

pub fn exec_on_message(worker: &Worker, mut f: impl FnMut(&web_sys::MessageEvent) + 'static) {
    let rc = Rc::new(RefCell::new(None));
    let rcc = rc.clone();
    let onmessage_closure = Closure::wrap(Box::new(move |event: Event| {
        if let Some(error) = event.dyn_ref::<ErrorEvent>() {
            klog!("error in worker: {}", error.message());
            // TODO: this probably leaks memory somehow? It's sort of
            // unclear what to do about errors in workers right now.
            return;
        }

        // If this is a completion event then can deallocate our own
        // callback by clearing out `slot2` which contains our own closure.
        if let Some(msg) = event.dyn_ref::<MessageEvent>() {
            f(msg);
            *rcc.borrow_mut() = None;
            return;
        }

        klog!("unhandled event: {}", event.type_());
        // TODO: like above, maybe a memory leak here?
    }) as Box<dyn FnMut(Event)>);
    worker.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));
    *rc.borrow_mut() = Some(onmessage_closure);
}

impl WorkerPool {
    /// Creates a new `WorkerPool` which immediately creates `initial` workers.
    ///
    /// The pool created here can be used over a long period of time, and it
    /// will be initially primed with `initial` workers. Currently workers are
    /// never released or gc'd until the whole pool is destroyed.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    //#[wasm_bindgen(constructor)]
    pub fn new(initial: usize) -> Result<WorkerPool, JsValue> {
        let pool = WorkerPool {
            state: Rc::new(PoolState {
                workers: RefCell::new(Vec::with_capacity(initial)),
                callback: Closure::wrap(Box::new(|event: Event| {
                    //klog!("unhandled event: {}", event.type_());
                    let array = js_sys::Array::new();
                    array.push(&JsValue::from_str("worker error (did it compile?):"));
                    array.push(&event);

                    web_sys::console::log(&array);

                    // {
                    //     let array = js_sys::Array::new();
                    //     array.push(&event);
                    //     web_sys::console::trace(&array);
                    // }
                }) as Box<dyn FnMut(Event)>),
            }),
        };
        for _ in 0..initial {
            let worker = spawn_worker()?;
            pool.state.push(worker);
        }

        Ok(pool)
    }

    /// Fetches a worker from this pool, spawning one if necessary.
    ///
    /// This will attempt to pull an already-spawned web worker from our cache
    /// if one is available, otherwise it will spawn a new worker and return the
    /// newly spawned worker.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    pub fn worker(&self) -> Result<Worker, JsValue> {
        match self.state.workers.borrow_mut().pop() {
            Some(worker) => Ok(worker),
            None => spawn_worker(),
        }
    }

    /// Executes the work `f` in a web worker, spawning a web worker if
    /// necessary.
    ///
    /// This will acquire a web worker and then send the closure `f` to the
    /// worker to execute. The worker won't be usable for anything else while
    /// `f` is executing, and no callbacks are registered for when the worker
    /// finishes.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    fn execute(&self, f: impl FnOnce() + Send + 'static) -> Result<Worker, JsValue> {
        execute_unpooled(&self.worker()?, f)
    }

    /// Configures an `onmessage` callback for the `worker` specified for the
    /// web worker to be reclaimed and re-inserted into this pool when a message
    /// is received.
    ///
    /// Currently this `WorkerPool` abstraction is intended to execute one-off
    /// style work where the work itself doesn't send any notifications and
    /// whatn it's done the worker is ready to execute more work. This method is
    /// used for all spawned workers to ensure that when the work is finished
    /// the worker is reclaimed back into this pool.
    fn reclaim_on_message(&self, worker: &Worker) {
        let state = Rc::downgrade(&self.state);
        let worker2 = worker.clone();

        exec_on_message(&worker, move |_msg| {
            if let Some(state) = state.upgrade() {
                state.push(worker2.clone());
            }
        });

        // let reclaim_slot = Rc::new(RefCell::new(None));
        // let slot2 = reclaim_slot.clone();
        // let reclaim = Closure::wrap(Box::new(move |event: Event| {
        //     if let Some(error) = event.dyn_ref::<ErrorEvent>() {
        //         klog!("error in worker: {}", error.message());
        //         // TODO: this probably leaks memory somehow? It's sort of
        //         // unclear what to do about errors in workers right now.
        //         return;
        //     }

        //     // If this is a completion event then can deallocate our own
        //     // callback by clearing out `slot2` which contains our own closure.
        //     if let Some(_msg) = event.dyn_ref::<MessageEvent>() {
        //         if let Some(state) = state.upgrade() {
        //             state.push(worker2.clone());
        //         }
        //         *slot2.borrow_mut() = None;
        //         return;
        //     }

        //     klog!("unhandled event: {}", event.type_());
        //     // TODO: like above, maybe a memory leak here?
        // }) as Box<dyn FnMut(Event)>);
        // worker.set_onmessage(Some(reclaim.as_ref().unchecked_ref()));
        // *reclaim_slot.borrow_mut() = Some(reclaim);
    }
}

impl WorkerPool {
    /// Executes `f` in a web worker.
    ///
    /// This pool manages a set of web workers to draw from, and `f` will be
    /// spawned quickly into one if the worker is idle. If no idle workers are
    /// available then a new web worker will be spawned.
    ///
    /// Once `f` returns the worker assigned to `f` is automatically reclaimed
    /// by this `WorkerPool`. This method provides no method of learning when
    /// `f` completes, and for that you'll need to use `run_notify`.
    ///
    /// # Errors
    ///
    /// If an error happens while spawning a web worker or sending a message to
    /// a web worker, that error is returned.
    pub fn run(&self, f: impl FnOnce() + Send + 'static) -> Result<(), JsValue> {
        let worker = self.execute(f)?;
        self.reclaim_on_message(&worker);
        Ok(())
    }
}

impl PoolState {
    fn push(&self, worker: Worker) {
        //worker.set_onmessage(Some(self.callback.as_ref().unchecked_ref()));
        worker.set_onerror(Some(self.callback.as_ref().unchecked_ref()));
        let mut workers = self.workers.borrow_mut();
        for prev in workers.iter() {
            let prev: &JsValue = prev;
            let worker: &JsValue = &worker;
            assert!(prev != worker);
        }
        workers.push(worker);
    }
}

/// Entry point invoked by `worker.js`, a bit of a hack but see the "TODO" above
/// about `worker.js` in general.
#[wasm_bindgen]
pub fn child_entry_point(ptr: u32) -> Result<(), JsValue> {
    let ptr = unsafe { Box::from_raw(ptr as *mut Work) };
    let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    global.post_message(&JsValue::undefined())?;
    (ptr.func)();
    global.post_message(&JsValue::undefined())?;
    Ok(())
}

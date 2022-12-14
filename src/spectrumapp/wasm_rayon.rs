#![cfg_attr(
    feature = "nightly",
    feature(external_doc),
    doc(include = "../README.md")
)]
#![cfg_attr(
    not(feature = "nightly"),
    doc = "Check out documentation in [README.md](https://github.com/GoogleChromeLabs/wasm-bindgen-rayon)."
)]

// Note: `atomics` is whitelisted in `target_feature` detection, but `bulk-memory` isn't,
// so we can check only presence of the former. This should be enough to catch most common
// mistake (forgetting to pass `RUSTFLAGS` altogether).
// #[cfg(not(target_feature = "atomics"))]
// compile_error!("Did you forget to enable `atomics` and `bulk-memory` features as outlined in wasm-bindgen-rayon README?");

use spmc::{channel, Receiver, Sender};
use web_sys::Worker;

/**
 * Copyright 2021 Google Inc. All Rights Reserved.
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *     http://www.apache.org/licenses/LICENSE-2.0
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use wasm_bindgen::prelude::*;

#[cfg(feature = "no-bundler")]
use js_sys::JsString;

use crate::klog;
use crate::spectrumapp::appstate::set_app_state;
use crate::spectrumapp::appstate::AppState;

static mut WASM_RAYON_STARTED: bool = false;
static mut WASM_RAYON_POOL_BUILDER: Option<Arc<KRayonPoolBuilder>> = None;

pub fn get_wasm_rayon_pool_builder() -> Option<Arc<KRayonPoolBuilder>> {
    unsafe { WASM_RAYON_POOL_BUILDER.clone() }
}

pub fn wasm_rayon_started() -> bool {
    unsafe { WASM_RAYON_STARTED }
}

pub fn init_wasm_rayon_legacy(rayon_threads: u32) {
    let wasm_rayon = KRayonPoolBuilder::new(rayon_threads as usize);
    KRayonPoolBuilder::spawn(&wasm_rayon);

    unsafe {
        WASM_RAYON_POOL_BUILDER = Some(wasm_rayon.clone());
    }
    Box::leak(Box::new(wasm_rayon));
}

#[allow(unused)]
pub fn simple_rayon_wasm_thread(tb: rayon::ThreadBuilder) {
    tb.run();
}
#[allow(unused)]
pub fn init_wasm_rayon_regular_spawn() {
    klog!("init with simple_rayon_wasm_thread");
    let _pool = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .spawn_handler(move |threadbr| {
            std::thread::spawn(|| simple_rayon_wasm_thread(threadbr));
            Ok(())
        })
        .build_global()
        .unwrap();
}

pub fn init_wasm_rayon(rayon_threads: u32) {
    init_wasm_rayon_legacy(rayon_threads);

    // wasm_rayon.borrow_mut().build();
    // let timeout_fn = move || {
    //     wasm_rayon.borrow_mut().build();
    // };
    // let callback = Closure::wrap(Box::new(timeout_fn) as Box<dyn FnMut()>);

    // web_sys::window()
    //     .unwrap()
    //     .set_timeout_with_callback_and_timeout_and_arguments_0(
    //         callback.as_ref().unchecked_ref(),
    //         100,
    //     )
    //     .unwrap();
    // callback.forget();
}

// Naming is a workaround for https://github.com/rustwasm/wasm-bindgen/issues/2429
// and https://github.com/rustwasm/wasm-bindgen/issues/1762.
#[allow(non_camel_case_types)]
#[wasm_bindgen]
#[doc(hidden)]
pub struct KRayonPoolBuilder {
    me: Weak<KRayonPoolBuilder>,

    pub num_threads: usize,
    #[wasm_bindgen(skip)]
    pub alive_threads: Arc<AtomicUsize>,

    sender: Mutex<Sender<rayon::ThreadBuilder>>,
    receiver: Receiver<rayon::ThreadBuilder>,
}

// #[cfg_attr(
//     not(feature = "no-bundler"),
//     wasm_bindgen(module = "/src/workerHelpers.js")
// )]
// #[cfg_attr(
//     feature = "no-bundler",
//     wasm_bindgen(module = "/src/workerHelpers.no-bundler.js")
// )]
// extern "C" {
//     #[wasm_bindgen(js_name = startWorkers)]
//     fn start_workers(module: JsValue, memory: JsValue, builder: wbg_rayon_PoolBuilder) -> Promise;
// }

impl KRayonPoolBuilder {
    pub fn new(num_threads: usize) -> Arc<Self> {
        let (sender, receiver) = channel();

        let pb = Arc::new_cyclic(|me| Self {
            me: me.clone(),
            num_threads,
            alive_threads: Arc::new(AtomicUsize::new(0)),
            sender: Mutex::new(sender),
            receiver,
        });
        pb
    }

    pub fn num_threads(&self) -> usize {
        self.num_threads
    }

    pub fn receiver(&self) -> Receiver<rayon::ThreadBuilder> {
        self.receiver.clone()
    }

    pub fn spawn(&self) {
        //let num_threads = selfref.lock().unwrap().num_threads;
        let num_threads = self.num_threads;
        klog!("rayon spawning WorkerPool");
        let js_pool = super::pool::WorkerPool::new(num_threads + 1)
            .map_err(|err| {
                //klog!("rayon spawning WorkerPool: {:?}", err);
                err
            })
            .unwrap();
        let last_idle_worker = js_pool.worker().unwrap();
        let notified_threads = Arc::new(AtomicU32::new(0));
        for _ in 0..num_threads {
            let js_worker = js_pool.worker().unwrap();
            let js_workerc = js_worker.clone();

            {
                let notified_threads = notified_threads.clone();
                let selfclone: Arc<Self> = self.me.upgrade().unwrap();

                let last_idle_worker = last_idle_worker.clone();
                super::pool::exec_on_message(&js_workerc, move |_msg| {
                    klog!("onmessage on main thread rayon");
                    let notified = 1 + notified_threads.fetch_add(1, Ordering::SeqCst);
                    if notified as usize == num_threads {
                        klog!("achieved all {} workers!", num_threads);
                        set_app_state(AppState::AwaitingLastWorker);
                        let last_idle_worker = last_idle_worker.clone();
                        selfclone.on_all_workers_started(last_idle_worker);
                    }
                });
            }
            {
                let receiver = self.receiver();
                let alive_threads = self.alive_threads.clone();
                super::pool::execute_unpooled(&js_workerc, move || {
                    alive_threads.fetch_add(1, Ordering::SeqCst);

                    receiver.recv().unwrap().run();
                })
                .unwrap();
            }
            Box::leak(Box::new(js_worker));
        }
        set_app_state(AppState::AwaitingRayonThreads);

        Box::leak(Box::new(js_pool));
    }

    pub fn on_all_workers_started(&self, last_idle_worker: Worker) {
        // .build() uses atomics so let's use workers again
        let selfclone = self.me.upgrade().unwrap();
        super::pool::execute_unpooled(&last_idle_worker, move || {
            klog!("executing rayon::build on worker");
            set_app_state(AppState::RayonBuildDone);
            selfclone.build();
        })
        .unwrap();
    }
    pub fn build(&self) {
        let alive = self.alive_threads.load(Ordering::SeqCst);
        if alive != self.num_threads {
            klog!(
                "warn: threads are not alive yet {} != {}",
                alive,
                self.num_threads
            );
        } else {
            klog!("building rayon: {} threads", self.num_threads);
        }
        rayon::ThreadPoolBuilder::new()
            .num_threads(self.num_threads)
            // We could use postMessage here instead of Rust channels,
            // but currently we can't due to a Chrome bug that will cause
            // the main thread to lock up before it even sends the message:
            // https://bugs.chromium.org/p/chromium/issues/detail?id=1075645
            .spawn_handler(move |threadbr| {
                // Note: `send` will return an error if there are no receivers.
                // We can use it because all the threads are spawned and ready to accept
                // messages by the time we call `build()` to instantiate spawn handler.
                self.sender.lock().unwrap().send(threadbr).unwrap();
                Ok(())
            })
            .build_global()
            .unwrap();

        unsafe {
            WASM_RAYON_STARTED = true;
        }
    }
}

// #[wasm_bindgen(js_name = initThreadPool)]
// #[doc(hidden)]
// pub fn init_thread_pool(num_threads: usize) -> Promise {
//     start_workers(
//         wasm_bindgen::module(),
//         wasm_bindgen::memory(),
//         wbg_rayon_PoolBuilder::new(num_threads),
//     )
// }

// #[wasm_bindgen]
// #[allow(clippy::not_unsafe_ptr_arg_deref)]
// #[doc(hidden)]
// pub fn wbg_rayon_start_worker(receiver: *const Receiver<rayon::ThreadBuilder>)
// where
//     // Statically assert that it's safe to accept `Receiver` from another thread.
//     Receiver<rayon::ThreadBuilder>: Sync,
// {
//     // This is safe, because we know it came from a reference to PoolBuilder,
//     // allocated on the heap by wasm-bindgen and dropped only once all the
//     // threads are running.
//     //
//     // The only way to violate safety is if someone externally calls
//     // `exports.wbg_rayon_start_worker(garbageValue)`, but then no Rust tools
//     // would prevent us from issues anyway.
//     let receiver = unsafe { &*receiver };
//     // Wait for a task (`ThreadBuilder`) on the channel, and, once received,
//     // start executing it.
//     //
//     // On practice this will start running Rayon's internal event loop.
//     receiver.recv().unwrap_throw().run()
// }

use super::appstate::set_app_state;
use super::appstate::AppState;
use super::appthread::PCMSender;
use super::dependent_module;

use crate::klog;

use web_sys::AudioWorkletNode;
use web_sys::AudioWorkletNodeOptions;
use web_sys::MediaStreamTrack;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::AudioBuffer;
use web_sys::AudioBufferSourceNode;
use web_sys::AudioContext;
use web_sys::MediaStream;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Weak;

thread_local! {
    pub static AUDIO_CONTEXT: RefCell<Option<AudioContext>> = RefCell::new(None);
}

pub fn global_audio_context() -> Result<AudioContext, JsValue> {
    AUDIO_CONTEXT.with(|optional_ctx| {
        if (*optional_ctx.borrow_mut()).is_none() {
            let ctx = web_sys::AudioContext::new()?;

            let cloned = ctx.clone();
            *optional_ctx.borrow_mut() = Some(ctx);
            Ok(cloned)
        } else {
            Ok((*optional_ctx.borrow_mut()).as_ref().unwrap().clone())
        }
    })
}

pub struct AdeviceWeb {
    me: Weak<AdeviceWeb>,
    pcm_sender: Box<dyn PCMSender>,
    pub source: Option<AudioBufferSourceNode>,
    pub reference_audio_buffer: Option<AudioBuffer>,
    pub playing_ref: bool,
    pub rolling_gain: f64,
    pub current_power: f64,
}

impl AdeviceWeb {
    pub fn new(pcm_sender: Box<dyn PCMSender>) -> Arc<Self> {
        let adevice = Arc::new_cyclic(|me| Self {
            me: me.clone(),
            reference_audio_buffer: None,
            source: None,
            playing_ref: false,
            rolling_gain: 0.0001,
            current_power: 1.0,
            pcm_sender,
        });
        adevice
    }

    pub fn start(&self) -> Result<(), JsValue> {
        set_app_state(AppState::StartAudioCapture);
        let fut = Self::start_capture_try(self.me.upgrade().unwrap());

        wasm_bindgen_futures::spawn_local(fut);

        Ok(())
    }

    pub async fn start_capture_try(self: Arc<Self>) -> () {
        let result = Self::start_capture(self).await;
        match result {
            Err(err) => {
                set_app_state(AppState::GetUserMediaFailed);
                klog!("start_capture err: {:?}", err);
            }
            Ok(ok) => {
                klog!("start_capture ok: {:?}", ok);
            }
        }
    }

    pub async fn start_capture(self: Arc<Self>) -> Result<JsValue, JsValue> {
        set_app_state(AppState::StartAudioCaptureAsync);

        klog!("getting audio devices");
        let window = web_sys::window().unwrap();

        let nav = window.navigator();
        let devices = nav.media_devices()?;

        if devices.is_undefined() {
            return Err(JsValue::from_str("mediaDevices undefined"));
        }

        let mut constraints = web_sys::MediaStreamConstraints::new();

        let mut c = web_sys::MediaTrackConstraints::new();
        c.echo_cancellation(&JsValue::FALSE);
        c.noise_suppression(&JsValue::FALSE);
        c.auto_gain_control(&JsValue::FALSE);
        constraints.audio(&c);
        constraints.video(&JsValue::FALSE);

        let success_promise = devices.get_user_media_with_constraints(&constraints)?;

        set_app_state(AppState::WaitingForUserAudio);

        let maybe_stream = JsFuture::from(success_promise).await?;

        let _result = self.on_media_stream_acquired(maybe_stream).await?;

        Ok(JsValue::null())
    }

    pub async fn on_media_stream_acquired(&self, maybe_stream: JsValue) -> Result<(), JsValue> {
        set_app_state(AppState::MediaStreamTrack);

        let media_stream: MediaStream = maybe_stream.dyn_into()?;
        let tracks_arr = media_stream.get_audio_tracks();

        let media_stream_track: MediaStreamTrack = tracks_arr.get(0).dyn_into()?;
        let mut c = web_sys::MediaTrackConstraints::new();
        c.echo_cancellation(&JsValue::FALSE);
        c.noise_suppression(&JsValue::FALSE);
        c.auto_gain_control(&JsValue::FALSE);

        let promise = media_stream_track.apply_constraints_with_constraints(&c)?;

        let _what = JsFuture::from(promise).await?;

        self.on_media_stream_acquired_prepared(media_stream).await?;

        Ok(())
    }

    #[allow(unused)]
    pub fn oneshot_callback<F: 'static>(
        mut callback: F,
    ) -> Rc<RefCell<Option<Closure<dyn FnMut(JsValue)>>>>
    where
        F: FnMut(JsValue) -> Result<(), JsValue>,
    {
        let rc = Rc::new(RefCell::new(None));
        let rcc = rc.clone();

        let cb_wrapper = move |value: JsValue| {
            callback(value);
            // free memory
            *rcc.borrow_mut() = None;
        };
        *rc.borrow_mut() = Some(Closure::wrap(
            Box::new(cb_wrapper) as Box<dyn FnMut(JsValue)>
        ));
        rc
    }

    pub fn create_worklet_processor_node(
        &self,
        ctx: &AudioContext,
        process: Box<dyn FnMut(&[f32]) -> bool>,
    ) -> Result<AudioWorkletNode, JsValue> {
        AudioWorkletNode::new_with_options(
            &ctx,
            "WasmProcessor",
            &AudioWorkletNodeOptions::new().processor_options(Some(&js_sys::Array::of3(
                &wasm_bindgen::module(),
                &wasm_bindgen::memory(),
                &WasmAudioProcessor {
                    process_fn: process,
                }
                .pack()
                .into(),
            ))),
        )
    }

    pub async fn prepare_wasm_audio(ctx: &AudioContext) -> Result<(), JsValue> {
        //polyfill_nop();
        let mod_url = dependent_module::on_the_fly(include_str!("js/kworklet.js")).await?;
        JsFuture::from(ctx.audio_worklet()?.add_module(&mod_url)?).await?;
        Ok(())
    }

    pub async fn on_media_stream_acquired_prepared(
        &self,
        media_stream: MediaStream,
    ) -> Result<(), JsValue> {
        let me = self.me.upgrade().unwrap().clone();
        set_app_state(AppState::PreparingWasmWorker);

        let atomic_started = Arc::new(AtomicBool::new(false));

        let process_fn = Box::new(move |samples: &[f32]| -> bool {
            if !atomic_started.swap(true, Ordering::Relaxed) {
                set_app_state(AppState::Playing);
            }

            me.pcm_sender.send_pcm(1, samples);
            true
        });
        let ctx = global_audio_context()?;

        let source = ctx.create_media_stream_source(&media_stream)?;

        Self::prepare_wasm_audio(&ctx).await?;
        let processor = self.create_worklet_processor_node(&ctx, process_fn)?;
        source.connect_with_audio_node(&processor)?;

        let destination = ctx.create_media_stream_destination()?;

        processor.connect_with_audio_node(&destination.dyn_into()?)?;

        klog!("we got microphone media device");

        Ok(())
    }
}

#[wasm_bindgen]
pub struct WasmAudioProcessor {
    process_fn: Box<dyn FnMut(&[f32]) -> bool>,
}

#[wasm_bindgen]
impl WasmAudioProcessor {
    pub fn process(&mut self, buf: &[f32]) -> bool {
        (self.process_fn)(buf)
    }
    pub fn pack(self) -> usize {
        Box::into_raw(Box::new(self)) as usize
    }
    pub unsafe fn unpack(val: usize) -> Self {
        *Box::from_raw(val as *mut _)
    }
}

// // TextEncoder and TextDecoder are not available in Audio Worklets, but there
// // is a dirty workaround: Import polyfill.js to install stub implementations
// // of these classes in globalThis.
// #[wasm_bindgen(module = "/src/spectrumapp/polyfill.js")]
// extern "C" {
//     fn polyfill_nop();
// }

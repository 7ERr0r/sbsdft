use super::PCMReceiver;

use web_sys::MediaStreamTrack;
use web_sys::ScriptProcessorNode;
use web_sys::WorkletOptions;
use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crate::klog;

use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::AudioBuffer;
use web_sys::AudioBufferSourceNode;
use web_sys::AudioContext;
use web_sys::AudioProcessingEvent;
use web_sys::MediaStream;

use std::cell::RefCell;
use std::mem::{self, MaybeUninit};
use std::rc::Rc;
use std::rc::Weak;

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
    grapher: Weak<RefCell<AdeviceWeb>>,
    channels: Option<Box<dyn PCMReceiver>>,
    //pub r_channel: SlidingImpl,
    pub source: Option<AudioBufferSourceNode>,
    pub reference_audio_buffer: Option<AudioBuffer>,
    pub playing_ref: bool,
    pub rolling_gain: f64,
    pub current_power: f64,
    tx: Option<Sender<Vec<f32>>>,
    //uint8buf: js_sys::Uint8Array,
}

impl AdeviceWeb {
    pub fn new(channels: Box<dyn PCMReceiver>) -> Rc<RefCell<Self>> {
        // SAFETY: ? initialized below
        let rc = Rc::new(RefCell::new(MaybeUninit::uninit()));

        *rc.borrow_mut() = MaybeUninit::new(Self {
            grapher: unsafe { mem::transmute(Rc::downgrade(&rc)) },
            reference_audio_buffer: None,
            source: None,
            playing_ref: false,
            rolling_gain: 0.0001,
            current_power: 1.0,
            channels: Some(channels),
            tx: None,
        });

        unsafe { mem::transmute(rc) }
    }

    // pub fn move_probes(&mut self, left: bool) {
    //     match &mut self.l_channel {
    //         SlidingImpl::DFT(dft) => dft.move_probes(left),
    //         SlidingImpl::Correlator(corr) => corr.move_probes(left),
    //     };
    // }

    pub fn start(&mut self) -> Result<(), JsValue> {
        self.start_capture()?;

        use crossbeam_channel::bounded;
        let (tx, rx) = bounded::<Vec<f32>>(1024);
        let mut channels: Option<Box<dyn PCMReceiver>> = None;
        let mut tx = Some(tx);
        std::mem::swap(&mut channels, &mut self.channels);
        std::mem::swap(&mut self.tx, &mut tx);

        super::kwasm::spawn_once(move || {
            Self::receiver_task(rx, channels.unwrap());
        });

        Ok(())
    }
    pub fn receiver_task(rx: Receiver<Vec<f32>>, out_channels: Box<dyn PCMReceiver>) {
        let mut bufs: Vec<Vec<f32>> = Vec::with_capacity(out_channels.num_channels() as usize);

        for _i in 0..out_channels.num_channels() {
            bufs.push(Vec::with_capacity(1024));
        }
        loop {
            match rx.recv() {
                Err(_) => break,
                Ok(samples) => {
                    Self::on_receive(&mut bufs, 2, &out_channels, &samples);
                }
            }
        }
    }
    pub fn on_receive(
        bufs: &mut Vec<Vec<f32>>,
        in_channels: usize,
        out_channels: &Box<dyn PCMReceiver>,
        samples: &[f32],
    ) {
        for c in 0..out_channels.num_channels() {
            let mut buf = &mut bufs[c];
            buf.resize(samples.len() / in_channels, 0.0);
            let buf = &mut buf;

            // 0 or 1 offset
            let mut i = c;
            let mut j = 0;
            while i < samples.len() {
                buf[j] = samples[i];
                i += in_channels;
                j += 1;
            }
        }

        out_channels.on_receive(&bufs);
    }

    fn start_capture(&mut self) -> Result<(), JsValue> {
        let window = web_sys::window().unwrap();

        let nav = window.navigator();
        let devices = match nav.media_devices() {
            Ok(devices) => {
                if devices.is_undefined() {
                    Err(JsValue::from_str("mediaDevices undefined"))
                } else {
                    Ok(devices)
                }
            }
            Err(err) => Err(err),
        }?;
        /*
        match devices {
            Ok(devices) => devices,
            Err(err) =>{
                console_warn!("no mediaDevices (no https?): {:?}", err);
                return Ok(())
            },
        }?;*/
        let mut constraints = web_sys::MediaStreamConstraints::new();

        let mut c = web_sys::MediaTrackConstraints::new();
        c.echo_cancellation(&JsValue::FALSE);
        c.noise_suppression(&JsValue::FALSE);
        c.auto_gain_control(&JsValue::FALSE);
        constraints.audio(&c);
        constraints.video(&JsValue::FALSE);

        //constraints.picture(false);
        //let success_promise = devices.get_user_media_with_constraints(&constraints)?;

        let success_promise = devices.get_user_media_with_constraints(&constraints)?;

        let f = Rc::new(RefCell::new(None));
        let ff = f.clone();

        let weak = self.grapher.clone();
        let success_cb = move |maybe_stream: JsValue| {
            weak.upgrade().map(|strong| {
                match strong.borrow_mut().on_media_stream_acquired(maybe_stream) {
                    Ok(x) => x,
                    Err(err) => {
                        klog!("stream in callback is not a MediaStream: {:?}", err);
                    }
                };
            });
            drop(f.borrow().as_ref().unwrap());
        };

        *ff.borrow_mut() = Some(Closure::wrap(
            Box::new(success_cb) as Box<dyn FnMut(JsValue)>
        ));
        let _unused = success_promise.then(ff.borrow().as_ref().unwrap());

        Ok(())
    }

    // pub fn toggle_reference_player(&mut self) -> Result<(), JsValue> {
    //     let ctx = global_audio_context()?;

    //     if self.source.is_none() {
    //         let source = ctx.create_buffer_source()?;
    //         source.set_buffer(self.reference_audio_buffer.as_ref());
    //         source.connect_with_audio_node(&ctx.destination())?;
    //         self.source = Some(source);
    //     }

    //     match &self.source {
    //         None => {}
    //         Some(source) => {
    //             self.playing_ref = !self.playing_ref;
    //             if self.playing_ref {
    //                 klog!("starting ref player");
    //                 source.set_loop(true);
    //                 source.start_with_when(0.0)?;
    //             } else {
    //                 klog!("stopping ref player");
    //                 source.stop_with_when(0.0)?;
    //                 self.source = None;
    //             }
    //         }
    //     };
    //     Ok(())
    // }

    /*
    pub fn fetch_reference_signal(&mut self) {
        let weak = self.grapher.clone();
        let success = move |audio_buffer: web_sys::AudioBuffer| {
            let samples_f32 = audio_buffer.get_channel_data(0).unwrap();

            let samples_i16: Vec<i16> = samples_f32.iter().map(|x| (x * 32767.0) as i16).collect();

            console_log!("reference buf len: {:?}", samples_i16.len());

            weak.upgrade().map(|strong| {
                let mut corr = strong.borrow_mut();
                corr.reference_audio_buffer = Some(audio_buffer);
                corr.l_channel.reset();
                //corr.l_channel.reference = samples_i16;

                corr.l_channel.reference = load_ref_noise();
            });
        };

        let future = SlidingGrapher::fetch_decode_audio(Box::new(success));
        let _promise = future_to_promise(future);
    }*/

    // pub async fn fetch_decode_audio(
    //     success: Box<dyn FnMut(web_sys::AudioBuffer)>,
    // ) -> Result<JsValue, JsValue> {
    //     let audio_file = crate::request_raw_bytes("noise.wav").await?;
    //     let _promise = SlidingGrapher::decode_audio_data(&audio_file, success)?;
    //     Ok(wasm_bindgen::JsValue::null())
    // }

    pub fn decode_audio_data(
        audio_file: &[u8],
        mut success: Box<dyn FnMut(web_sys::AudioBuffer)>,
    ) -> Result<Promise, JsValue> {
        let ctx = global_audio_context()?;

        let cb_ondecode_success = move |maybe_audiobuffer: JsValue| -> Result<(), JsValue> {
            let audiobuffer: web_sys::AudioBuffer = maybe_audiobuffer.dyn_into()?;
            success(audiobuffer);
            Ok(())
        };
        let cb = Closure::wrap(
            Box::new(cb_ondecode_success) as Box<dyn FnMut(JsValue) -> Result<(), JsValue>>
        );
        let content_js_array =
            js_sys::Uint8Array::new(unsafe { &js_sys::Uint8Array::view(audio_file) });

        let promise = ctx.decode_audio_data_with_success_callback(
            &content_js_array.buffer(),
            cb.as_ref().unchecked_ref(),
        )?;
        cb.forget();
        Ok(promise)
    }

    pub fn on_media_stream_acquired(&mut self, maybe_stream: JsValue) -> Result<(), JsValue> {
        //let media_stream: MediaStream = maybe_stream.dyn_into();

        //self.fetch_reference_signal();

        let media_stream: MediaStream = maybe_stream.dyn_into()?;
        let tracks_arr = media_stream.get_audio_tracks();

        let media_stream_track: MediaStreamTrack = tracks_arr.get(0).dyn_into()?;
        let mut c = web_sys::MediaTrackConstraints::new();
        c.echo_cancellation(&JsValue::FALSE);
        c.noise_suppression(&JsValue::FALSE);
        c.auto_gain_control(&JsValue::FALSE);

        let promise = media_stream_track.apply_constraints_with_constraints(&c)?;

        let f = Rc::new(RefCell::new(None));
        let ff = f.clone();

        let weak = self.grapher.clone();
        let success_cb = move |_| {
            let media_streamc = media_stream.clone();
            weak.upgrade().map(|strong| {
                match strong
                    .borrow_mut()
                    .on_media_stream_acquired_prepared(media_streamc)
                {
                    Ok(x) => x,
                    Err(err) => {
                        klog!("on_media_stream_acquired_prepared: {:?}", err);
                    }
                };
            });
            drop(f.borrow().as_ref().unwrap());
        };

        *ff.borrow_mut() = Some(Closure::wrap(
            Box::new(success_cb) as Box<dyn FnMut(JsValue)>
        ));
        let _unused = promise.then(ff.borrow().as_ref().unwrap());
        Ok(())
    }

    pub fn oneshot_callback<F: 'static>(mut callback: F)
    where
        F: FnMut(JsValue),
    {
        let rc = Rc::new(RefCell::new(None));
        let rcc = rc.clone();

        let cb_wrapper = move |value: JsValue| {
            callback(value);
            *rcc.borrow_mut() = None;
        };
        *rc.borrow_mut() = Some(Closure::wrap(
            Box::new(cb_wrapper) as Box<dyn FnMut(JsValue)>
        ));
    }

    pub fn create_legacy_processor_node(
        &self,
        ctx: &AudioContext,
    ) -> Result<ScriptProcessorNode, JsValue> {
        let processor = ctx.create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(1024/4, 1, 1)?;

        Ok(processor)
    }

    pub fn create_worklet_processor_node(
        &self,
        ctx: &AudioContext,
    ) -> Result<ScriptProcessorNode, JsValue> {
        let processor = ctx.create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(1024/4, 1, 1)?;

        let worklet = ctx.audio_worklet()?;
        let mut options = WorkletOptions::new();

        options.credentials(web_sys::RequestCredentials::SameOrigin);
        let module = worklet.add_module_with_options("processor.js", &options)?;

        Ok(processor)
    }

    pub fn on_media_stream_acquired_prepared(
        &mut self,
        media_stream: MediaStream,
    ) -> Result<(), JsValue> {
        let ctx = global_audio_context()?;

        let source = ctx.create_media_stream_source(&media_stream)?;
        let processor = self.create_legacy_processor_node(&ctx)?;
        source.connect_with_audio_node(&processor)?;

        let destination = ctx.create_media_stream_destination()?;
        //let destination = ctx.destination()?;

        processor.connect_with_audio_node(&destination.dyn_into()?)?;

        /*
        let mut correlator = Correlator {
            uint8buf: js_sys::Uint8Array::new_with_length(1024),
        };
        */
        let weak = self.grapher.clone();
        let cb_onaudioprocess = move |maybe_processingevent: JsValue| -> Result<(), JsValue> {
            weak.upgrade().map_or(Ok(()), |strong| {
                if crate::spectrumapp::panicked() {
                    return Ok(());
                }
                strong.borrow_mut().onaudioprocess(maybe_processingevent)
            })
        };

        let cb = Closure::wrap(
            Box::new(cb_onaudioprocess) as Box<dyn FnMut(JsValue) -> Result<(), JsValue>>
        );
        processor.set_onaudioprocess(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
        klog!("we got microphone media device");

        Ok(())
    }

    pub fn onaudioprocess(&mut self, maybe_processingevent: JsValue) -> Result<(), JsValue> {
        let processingevent: AudioProcessingEvent = maybe_processingevent.dyn_into()?;

        let in_buf = processingevent.input_buffer()?;
        //console_log!("{:?}", in_buf);
        //let mut buf = Vec::with_capacity(1024);

        // SAFETY: init below
        //unsafe {
        //    buf.set_len(1024);
        //}
        //in_buf.copy_from_channel(buf.as_mut_slice(), 1)?;

        let buf = in_buf.get_channel_data(0)?;
        //console_log!("len: {}", in_buf.length());

        //console_log!("samples: {:?}", buf);

        // match &mut self.l_channel {
        //     SlidingImpl::DFT(dft) => dft.on_input(&buf),
        //     SlidingImpl::Correlator(corr) => corr.on_input(&buf),
        // };

        self.tx.as_ref().map(|tx| tx.try_send(buf));

        // let mut bufs = Vec::new();
        // bufs.push(buf);
        // match &self.channels {
        //     Some(channels) => {
        //         channels.on_receive(&bufs);
        //     }
        //     None => {

        //     }
        // }

        Ok(())
    }
}

use crate::spectrumapp::appstate::{set_app_state, AppState};

use super::MyParams;
use std::sync::Arc;
use std::sync::{Mutex, Weak};

// extern crate anyhow;
// extern crate cpal;
// extern crate hound;

use super::appthread::PCMSender;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;

fn start_cpal_stream(
    tx: Arc<dyn PCMSender>,
    audio_device_hint: &Option<String>,
) -> Result<cpal::Stream, anyhow::Error> {
    // Use the default host for working with audio devices.

    println!("start_cpal_stream");
    let host = cpal::default_host();

    // Setup the default input device and stream with the default input config.

    let mut descbuf: Vec<u8> = Vec::new();
    use std::io::Write;

    let device = match audio_device_hint {
        Some(hint) => {
            let hint = hint.to_lowercase();
            let devices = host.devices()?;
            writeln!(&mut descbuf, "Devices: ").unwrap();

            let mut found_device = None;
            for (device_index, device) in devices.enumerate() {
                if let Ok(conf) = device.default_input_config() {
                    match device.name() {
                        Ok(name) => {
                            writeln!(&mut descbuf, " {}", name).unwrap();
                            writeln!(&mut descbuf, "   {:?}", conf).unwrap();
                            if name.to_lowercase().contains(hint.as_str()) {
                                found_device = Some(device);
                                //break;
                            }
                        }
                        Err(err) => {
                            writeln!(&mut descbuf, " err: {}", err).unwrap();
                        }
                    }
                } else {
                    writeln!(
                        &mut descbuf,
                        "device with device_index:{} name:{} has no .default_input_config()",
                        device.name().unwrap_or("[noname]".to_string()),
                        device_index
                    )
                    .unwrap();
                }
            }
            found_device
        }
        None => {
            let device = host
                .default_input_device()
                .expect("Failed to get default input device");
            println!("Default input device: {}", device.name()?);
            Some(device)
        }
    };

    std::io::stdout().write_all(&descbuf[..])?;

    let device = device.expect("device by name not found");

    let def_config = device
        .default_input_config()
        .expect("Failed to get default input config");
    println!("Default input config: {:?}", def_config);
    // let config = cpal::StreamConfig{
    //     channels: 2,
    //     sample_rate: cpal::SampleRate(48000),
    //     buffer_size: cpal::BufferSize::Fixed(256),
    // };
    //let config = def_config;
    let config = def_config;
    //config.buffer_size = 1024;

    // A flag to indicate that recording is in progress.
    println!("Begin recording...");

    // Run the input stream on a separate thread.
    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let mut sine_t: f32 = 0.0;
    let mut t: f32 = 0.0;
    let tau = std::f32::consts::PI * 2.0;
    let mut debug_getter = move || -> f32 {
        t += tau * 0.1 / 48000.0;
        if t > tau {
            t -= tau;
        }
        sine_t += tau * (440.0 + 320.0 * t.sin()) / 48000.0;
        if sine_t > tau {
            sine_t -= tau;
        }
        sine_t.sin() / 2.0
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<f32, _>(data, tx.clone(), &mut debug_getter),
            err_fn,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i16, _>(data, tx.clone(), &mut debug_getter),
            err_fn,
        )?,
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<u16, _>(data, tx.clone(), &mut debug_getter),
            err_fn,
        )?,
    };

    set_app_state(AppState::Playing);
    stream.play()?;

    Ok(stream)
}

fn write_input_data<T, G>(input: &[T], tx: Arc<dyn PCMSender>, _debug_getter: &mut G)
where
    T: cpal::Sample,
    G: FnMut() -> f32,
{
    let mut v = Vec::with_capacity(input.len());

    // for &sample in input.iter().step_by(2) {
    //     //let sample: U = cpal::Sample::from(&sample);
    //     v.push(sample.to_f32());
    //     //v.push(debug_getter());
    // }

    for &sample in input {
        v.push(sample.to_f32());
    }

    //v.clone_from_slice(input);

    let _result = tx.send_pcm(-1, v);
}

pub struct SlidingCpal {
    me: Weak<Self>,
    sender: Arc<dyn PCMSender>,
    pub audio_device: Option<String>,
    pub stream: Mutex<Option<Stream>>,
}

pub fn resize_vec_len_fast<V>(v: &mut Vec<V>, new_len: usize)
where
    V: Clone + Default,
{
    if v.capacity() < new_len {
        v.resize(new_len, V::default());
    } else {
        unsafe {
            v.set_len(new_len);
        }
    }
}
impl SlidingCpal {
    pub fn new(sender: Arc<dyn PCMSender>, params: &MyParams) -> Arc<Self> {
        // SAFETY: ? initialized below
        let sc = Arc::new_cyclic(|me| Self {
            me: me.clone(),
            //self_weak: unsafe { mem::transmute(Rc::downgrade(&rc)) },
            sender,
            audio_device: params.audio_device.clone(),
            stream: Mutex::new(None),
        });

        //*rc.borrow_mut() = MaybeUninit::new();

        sc
    }

    pub fn start(&self) {
        //let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(1024);
        //let mut channels: Option<Box<dyn PCMSender>> = None;
        //std::mem::swap(&mut channels, &mut self.channels);
        let audio_device = self.audio_device.clone();

        let sender = self.sender.clone();

        let me = self.me.upgrade().unwrap();
        // std::thread::spawn(move || {

        //     //drop(stream);
        // });

        let stream = start_cpal_stream(sender, &audio_device).unwrap();
        //Self::receiver_task(rx, channels.unwrap());
        {
            let mut guard = me.stream.lock().unwrap();
            *guard = Some(stream);
        }
        // todo
        // #[cfg(target_arch = "wasm32")]
        // {
        //     let stream = start_writer(tx, &audio_device).unwrap();
        //     Self::receiver_task(rx, channels.unwrap());
        //     drop(stream);
        // }
    }

    // pub fn receiver_task(rx: Receiver<Vec<f32>>, out_channels: Box<dyn PCMReceiver>) {
    //     let mut bufs: Vec<Vec<f32>> = Vec::with_capacity(out_channels.num_channels() as usize);

    //     for _i in 0..out_channels.num_channels() {
    //         bufs.push(Vec::with_capacity(1024));
    //     }
    //     loop {
    //         match rx.recv() {
    //             Err(_) => break,
    //             Ok(samples) => {
    //                 Self::on_receive(&mut bufs, 2, &out_channels, &samples);
    //             }
    //         }
    //     }
    // }

    // pub fn on_receive(
    //     bufs: &mut Vec<Vec<f32>>,
    //     in_channels: usize,
    //     out_channels: &Box<dyn PCMReceiver>,
    //     samples: &[f32],
    // ) {
    //     //println!("on_receive len: {}", samples.len());
    //     //let out_num = out_channels.len();
    //     for c in 0..out_channels.num_channels() {
    //         let mut buf = &mut bufs[c];
    //         let new_len = samples.len() / in_channels;
    //         resize_vec_len_fast(&mut buf, new_len);

    //         let buf = &mut buf;

    //         // 0 or 1 offset
    //         let mut i = c;
    //         let mut j = 0;
    //         while i < samples.len() {
    //             buf[j] = samples[i];
    //             i += in_channels;
    //             j += 1;
    //         }
    //     }

    //     out_channels.on_receive(&bufs);

    //     // let channels_slice: &[i32] = (0..out_channels.num_channels()).map(|i|{
    //     //     0
    //     // }).collect();
    //     // for c in 0..out_channels.len() {
    //     //     let out_channel = &out_channels[c];
    //     //     let samples = &bufs[c];
    //     //     Self::on_receive_channel(out_channel, samples);
    //     // }
    // }
    // pub fn on_receive_channel(out_channel: &Weak<Mutex<SlidingImpl>>, samples: &[f32]) {
    //     out_channel.upgrade().map(|strong| {
    //         match &mut *strong.lock() {
    //             SlidingImpl::DFT(dft) => dft.on_input(&samples),
    //             //SlidingImpl::Correlator(corr) => corr.on_input(&buf),
    //             _ => {}
    //         };
    //     });
    // }
}

use std::sync::{Arc, Mutex, Weak};

use cpal::Sample;
use crossbeam_channel::{bounded, Receiver, Sender};

use super::sbswdft::SlidingImpl;

pub trait PCMSender: Send + Sync {
    fn num_channels(&self) -> usize;
    //fn on_receive(&self, samples: &[Vec<f32>]);

    fn send_pcm(&self, channel: i32, samples: &[f32]);
    fn send_pcm16(&self, channel: i32, samples: &[i16]);
}

#[derive(Clone)]
pub struct SlidingAppSender {
    main_pcm_tx: Sender<AppMsg>,

    /// We don't want to allocate on Audio worklet thread
    //reuse_buffers_tx: Sender<Vec<f32>>,
    reuse_buffers_rx: Receiver<Vec<f32>>,
    channel_num: usize,
}
impl SlidingAppSender {
    pub fn new(tx: Sender<AppMsg>, channel_num: usize, reuse_rx: Receiver<Vec<f32>>) -> Self {
        Self {
            channel_num,
            main_pcm_tx: tx,
            reuse_buffers_rx: reuse_rx,
            //weak_sliding_channels: channels,
        }
    }
}
impl SlidingAppSender {
    fn send_vec(&self, num_channels: i32, buf: Vec<f32>) {
        let _ = self
            .main_pcm_tx
            .try_send(AppMsg::PcmAudio(num_channels, buf));
    }
}

fn fill_buf_from_i16(buf: &mut Vec<f32>, samples: &[i16]) {
    buf.extend(samples.iter().map(|s| s.to_f32()));
}


impl PCMSender for SlidingAppSender {
    fn num_channels(&self) -> usize {
        self.channel_num
    }

    fn send_pcm(&self, num_channels: i32, samples: &[f32]) {
        let optbuf = self.reuse_buffers_rx.try_recv().ok();
        let buf: Vec<f32>;

        if let Some(mut sbuf) = optbuf {
            sbuf.clear();
            sbuf.reserve(samples.len());
            sbuf.extend_from_slice(samples);
            buf = sbuf;
        } else {
            buf = samples.to_vec();
        }

        self.send_vec(num_channels, buf);
    }

    fn send_pcm16(&self, num_channels: i32, samples: &[i16]) {
        let optbuf = self.reuse_buffers_rx.try_recv().ok();
        let buf: Vec<f32>;

        if let Some(mut sbuf) = optbuf {
            sbuf.clear();
            sbuf.reserve(samples.len());
            fill_buf_from_i16(&mut sbuf, samples);
            buf = sbuf;
        } else {
            let mut sbuf = Vec::with_capacity(samples.len());
            fill_buf_from_i16(&mut sbuf, samples);
            buf = sbuf;
        }

        self.send_vec(num_channels, buf);
    }

    // fn on_receive(&self, channels_samples: &[Vec<f32>]) {
    //     // for (out_channel, samples) in self.weak_sliding_channels.iter().zip(channels_samples) {
    //     //     out_channel.upgrade().map(|strong| {
    //     //         match &mut *strong.lock().unwrap() {
    //     //             SlidingImpl::DFT(dft) => dft.on_input(&samples),
    //     //             //SlidingImpl::Correlator(corr) => corr.on_input(&buf),
    //     //             //_ => {}
    //     //         };
    //     //     });
    //     // }
    // }
}

pub type AppFunc = dyn FnOnce(&ProcessingApp) + Send;

pub enum AppMsg {
    RunFunc(Box<AppFunc>),
    /// num_channels
    PcmAudio(i32, Vec<f32>),
}

pub struct ProcessingApp {
    #[allow(unused)]
    me: Weak<Self>,
    pub sliding_channels: Vec<Arc<Mutex<SlidingImpl>>>,
    bufs: Mutex<Vec<Vec<f32>>>,
    pub main_pcm_tx: Sender<AppMsg>,
    pub main_priority_tx: Sender<AppMsg>,

    /// We don't want to allocate on Audio worklet thread
    //pub reuse_buffers_tx: Sender<Vec<f32>>,
    pub reuse_buffers_rx: Receiver<Vec<f32>>,
    //pub reuse_buffers_tx: Sender<Vec<f32>>,
}

impl ProcessingApp {
    pub fn new(sliding_channels: Vec<Arc<Mutex<SlidingImpl>>>) -> Arc<Self> {
        let (tx, rx) = bounded(1024);
        let (reuse_tx, reuse_rx) = bounded(512);
        let (priority_tx, priority_rx) = bounded(8);

        let channels = sliding_channels.len();
        let mut bufs: Vec<Vec<f32>> = Vec::with_capacity(channels);

        for _i in 0..channels {
            bufs.push(Vec::with_capacity(1024));
        }

        let app = Arc::new_cyclic(|me| Self {
            me: me.clone(),
            sliding_channels: sliding_channels,
            bufs: Mutex::new(bufs),
            main_pcm_tx: tx,
            main_priority_tx: priority_tx,
            reuse_buffers_rx: reuse_rx,
            //reuse_buffers_tx: reuse_tx,
        });

        let appp = app.clone();
        super::kwasm::spawn_once("processingApp.main_thread", move || {
            appp.main_thread(priority_rx, rx, reuse_tx);
        });

        app
    }

    pub fn new_sender(&self) -> SlidingAppSender {
        SlidingAppSender::new(
            self.main_pcm_tx.clone(),
            self.sliding_channels.len(),
            self.reuse_buffers_rx.clone(),
        )
    }

    pub fn main_thread(
        &self,
        priority_rx: Receiver<AppMsg>,
        rx: Receiver<AppMsg>,
        reuse_tx: Sender<Vec<f32>>,
    ) {
        loop {
            if !self.main_loop(&priority_rx, &rx, &reuse_tx) {
                break;
            }
        }
    }

    pub fn main_loop(
        &self,
        priority_rx: &Receiver<AppMsg>,
        rx: &Receiver<AppMsg>,
        reuse_tx: &Sender<Vec<f32>>,
    ) -> bool {
        let mut msg = priority_rx.try_recv().ok();
        if msg.is_none() {
            msg = rx.recv().ok();
        }
        match msg {
            None => return false,
            Some(AppMsg::RunFunc(appfn)) => {
                appfn(self);
            }
            Some(AppMsg::PcmAudio(ch_num, samples)) => {
                let mut bufs = self.bufs.lock().unwrap();
                let in_channels = if ch_num < 0 {
                    self.sliding_channels.len()
                } else {
                    ch_num as usize
                };
                self.on_receive(&mut bufs, in_channels, &samples);

                // Reusing Vec<f32>
                let _ = reuse_tx.try_send(samples);
            }
        }

        true
    }

    pub fn on_receive(&self, bufs: &mut Vec<Vec<f32>>, in_channels: usize, samples: &[f32]) {
        for c in 0..in_channels {
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

        // TODO
        self.on_receive_sliding(&bufs);
    }

    fn on_receive_sliding(&self, channels_samples: &[Vec<f32>]) {
        for (out_channel, samples) in self.sliding_channels.iter().zip(channels_samples) {
            match &mut *out_channel.lock().unwrap() {
                SlidingImpl::DFT(dft) => dft.on_input(&samples),
                //SlidingImpl::Correlator(corr) => corr.on_input(&buf),
                //_ => {}
            };
        }
    }
}

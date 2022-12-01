use std::sync::{Arc, Mutex, Weak};

use crossbeam_channel::{bounded, Receiver, Sender};

use super::sbswdft::SlidingImpl;

pub trait PCMSender: Send + Sync {
    fn num_channels(&self) -> usize;
    //fn on_receive(&self, samples: &[Vec<f32>]);

    fn send_pcm(&self, channel: i32, samples: Vec<f32>);
}

pub enum AppMsg {
    RunFunc,
    /// num_channels
    PcmAudio(i32, Vec<f32>),
}

#[derive(Clone)]
pub struct SlidingAppSender {
    main_tx: Sender<AppMsg>,
    channel_num: usize,
}
impl SlidingAppSender {
    pub fn new(tx: Sender<AppMsg>, channel_num: usize) -> Self {
        Self {
            channel_num,
            main_tx: tx,
            //weak_sliding_channels: channels,
        }
    }
}

impl PCMSender for SlidingAppSender {
    fn num_channels(&self) -> usize {
        self.channel_num
    }

    fn send_pcm(&self, num_channels: i32, samples: Vec<f32>) {
        let _ = self.main_tx.try_send(AppMsg::PcmAudio(num_channels, samples));
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

pub struct ProcessingApp {
    #[allow(unused)]
    me: Weak<Self>,
    sliding_channels: Vec<Arc<Mutex<SlidingImpl>>>,
    bufs: Mutex<Vec<Vec<f32>>>,
    main_tx: Sender<AppMsg>,
}

impl ProcessingApp {
    pub fn new(sliding_channels: Vec<Arc<Mutex<SlidingImpl>>>) -> Arc<Self> {
        let (tx, rx) = bounded(1024);

        let channels = sliding_channels.len();
        let mut bufs: Vec<Vec<f32>> = Vec::with_capacity(channels);

        for _i in 0..channels {
            bufs.push(Vec::with_capacity(1024));
        }

        let app = Arc::new_cyclic(|me| Self {
            me: me.clone(),
            sliding_channels: sliding_channels,
            bufs: Mutex::new(bufs),
            main_tx: tx,
        });

        let appp = app.clone();
        super::kwasm::spawn_once(move || {
            appp.main_thread(rx);
        });

        app
    }

    pub fn new_sender(&self) -> SlidingAppSender {
        SlidingAppSender::new(self.main_tx.clone(), self.sliding_channels.len())
    }

    pub fn main_thread(&self, rx: Receiver<AppMsg>) {
        loop {
            if !self.main_loop(&rx) {
                break;
            }
        }
    }

    pub fn main_loop(&self, rx: &Receiver<AppMsg>) -> bool {
        match rx.recv() {
            Err(_) => return false,
            Ok(AppMsg::RunFunc) => {}
            Ok(AppMsg::PcmAudio(ch_num, samples)) => {
                let mut bufs = self.bufs.lock().unwrap();
                let in_channels = if ch_num < 0 {
                    self.sliding_channels.len()
                }else{
                    ch_num as usize
                };
                self.on_receive(&mut bufs, in_channels, &samples);
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

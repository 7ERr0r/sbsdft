use super::appstate::get_app_state;
use super::appstate::AppState;
use super::appthread::AppFunc;
use super::appthread::AppMsg;
use super::appthread::ProcessingApp;
use super::myvertex::*;
use super::sbswdft::ChannelSWDFT;
use super::sbswdft::Collected;
use super::sbswdft::SpectrumBins;
use super::sbswdft::SpectrumBinsState;
use super::sbswdft::WindowType;

use super::FontRenderer;
use super::GraphType;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::sync::Mutex;

use super::displayparams::DisplayParams;
use super::fontrenderer::FontAtlas;

use super::sbswdft::SlidingImpl;
use super::sbswdft::SpectrumConfig;

use std::convert::TryFrom;
use std::rc::Rc;
use std::sync::Arc;

pub struct SpectrumUI {
    pub app: Option<Arc<ProcessingApp>>,
    gui_scale: u32,
    divisions_hz: bool,

    sliding_impls: Vec<RefCell<SlidingChannel>>,

    display_params: DisplayParams,
    mouse_pos: cgmath::Vector2<f32>,
    _init_config: SpectrumConfig,
    zoom_config: SpectrumConfig,
    zoom: f32,
    graph_type: GraphType,
    dragging: bool,
    last_drag_pos: cgmath::Vector2<f32>,
    font_atlas: Rc<FontAtlas>,

    subdivisions: i32,
    logarithmic_scale: bool,
}

pub enum RendererMsg {
    NewSpectrum(Collected),
    ConfigUpdate(SpectrumConfig),
}

pub struct SlidingChannel {
    pub sliding_rc: Arc<Mutex<SlidingImpl>>,
    pub spectrum_receiver: Receiver<RendererMsg>,
    pub last_rolling_gain: f64,
    pub collected_spectrums: VecDeque<Collected>,
}

pub struct StateSnapshot {
    pub current_algo: SpectrumBinsState,
    pub window_type: WindowType,
    pub collect_every: usize,
    pub collect_frequency: usize,
    pub window_kernel_len: usize,
}

impl SpectrumUI {
    pub fn new(
        config: SpectrumConfig,
        font_atlas: FontAtlas,
        sliding_impls: Vec<SlidingChannel>,
    ) -> Self {
        let celled = sliding_impls.into_iter().map(|s| RefCell::new(s)).collect();
        Self {
            app: None,
            gui_scale: 2,
            divisions_hz: true,

            _init_config: config.clone(),
            zoom_config: config,
            sliding_impls: celled,
            display_params: DisplayParams {
                gui_scale: 2,
                dx: 400,
                dy: 400,
                gui_dx: 200,
                gui_dy: 200,
            },
            mouse_pos: cgmath::vec2(0.0, 0.0),
            zoom: 0.0,
            graph_type: GraphType::Line,
            dragging: false,
            last_drag_pos: cgmath::vec2(0.0, 0.0),
            font_atlas: Rc::new(font_atlas),

            subdivisions: 6,
            logarithmic_scale: false,
        }
    }
    pub fn on_resize(&mut self, dx: u32, dy: u32) {
        let display = DisplayParams {
            gui_scale: self.gui_scale,
            dx,
            dy,

            gui_dx: dx / self.gui_scale,
            gui_dy: dy / self.gui_scale,
        };
        self.display_params = display
    }

    pub fn on_resolution_scale(&mut self, delta: f32) {
        let scale = (2.0 as f32).powf(delta / 10.0);

        self.run_main(Box::new(move |app| {
            for sliding_arc in &app.sliding_channels {
                let mut channel = sliding_arc.lock().unwrap();

                match &mut *channel {
                    SlidingImpl::DFT(dft) => {
                        dft.config.wave_cycles_resolution *= scale;
                    }
                }
            }
        }));

        self.reinit_spectrum();
    }

    pub fn run_main(&self, f: Box<AppFunc>) {
        if let Some(app) = &self.app {
            let _ = app.main_priority_tx.try_send(AppMsg::RunFunc(f));
        }
    }

    pub fn move_spectrum(&mut self, left: bool, amount: i32) {
        self.run_main(Box::new(move |app| {
            for sliding_arc in &app.sliding_channels {
                let mut channel = sliding_arc.lock().unwrap();

                match &mut *channel {
                    SlidingImpl::DFT(dft) => {
                        dft.move_bins(left, amount);
                        //self.zoom_config = dft.config.clone();
                    }
                }
            }
        }));
    }

    pub fn toggle_peaks(&mut self) {
        let n: u8 = self.graph_type.into();
        self.graph_type = match GraphType::try_from(n + 1) {
            Ok(x) => x,
            Err(_) => GraphType::try_from(0).unwrap(),
        }
    }
    pub fn toggle_logarithmic(&mut self) {
        self.logarithmic_scale = !self.logarithmic_scale;
    }

    pub fn cycle_method(&mut self) {
        self.run_main(Box::new(move |app| {
            for sliding_arc in &app.sliding_channels {
                let mut channel = sliding_arc.lock().unwrap();

                match &mut *channel {
                    SlidingImpl::DFT(dft) => {
                        let mut method = match &dft.spectrum_bins {
                            SpectrumBins::DFT(_) => 0,
                            SpectrumBins::NC(_) => 1,
                        };
                        method = (method + 1) % 2;
                        let bins = ChannelSWDFT::make_spectrum_bins(method, &dft.config);
                        dft.spectrum_bins = bins;
                    }
                }
            }
        }));
    }

    pub fn toggle_pause(&mut self) {
        self.run_main(Box::new(move |app| {
            for sliding_arc in &app.sliding_channels {
                let mut channel = sliding_arc.lock().unwrap();
                match &mut *channel {
                    SlidingImpl::DFT(dft) => {
                        dft.paused = !dft.paused;
                    }
                }
            }
        }));
    }

    pub fn toggle_colorize(&mut self) {
        self.run_main(Box::new(move |app| {
            for sliding_arc in &app.sliding_channels {
                let mut channel = sliding_arc.lock().unwrap();
                match &mut *channel {
                    SlidingImpl::DFT(dft) => {
                        dft.should_colorize = !dft.should_colorize;
                    }
                }
            }
        }));
    }

    pub fn toggle_subtraction_peaks(&mut self) {
        self.run_main(Box::new(move |app| {
            for sliding_arc in &app.sliding_channels {
                let mut channel = sliding_arc.lock().unwrap();
                match &mut *channel {
                    SlidingImpl::DFT(dft) => {
                        dft.config.subtraction_peaks = !dft.config.subtraction_peaks;
                        //self.zoom_config = dft.config.clone();
                    }
                }
            }
        }));
    }

    pub fn cycle_window_type(&mut self) {
        //self.win_id = (self.win_id + 1) % 3;
        self.run_main(Box::new(move |app| {
            for sliding_arc in &app.sliding_channels {
                let mut channel = sliding_arc.lock().unwrap();
                match &mut *channel {
                    SlidingImpl::DFT(dft) => {
                        let w: u8 = dft.collector.windowtype.into();

                        dft.collector.windowtype = match WindowType::try_from(w + 1) {
                            Ok(x) => x,
                            Err(_) => WindowType::try_from(0).unwrap(),
                        }
                    }
                }
            }
        }));
        self.reset_window_kernel();
    }
    pub fn change_window_subdivisions(&mut self, more: bool) {
        if more {
            self.subdivisions += 1;
        } else {
            self.subdivisions -= 1;
        }

        if self.subdivisions <= 0 {
            self.subdivisions = 1;
        }
        self.reset_window_kernel();
    }

    pub fn change_collect_freq(&mut self, more: bool) {
        self.run_main(Box::new(move |app| {
            for sliding_arc in &app.sliding_channels {
                let mut channel = sliding_arc.lock().unwrap();

                match &mut *channel {
                    SlidingImpl::DFT(dft) => {
                        if more {
                            dft.set_collect_frequency(dft.collect_frequency + 60);
                        } else {
                            if dft.collect_frequency >= 120 {
                                let x = dft.collect_frequency - 60;
                                dft.set_collect_frequency(x);
                            }
                        }
                    }
                }
            }
        }));
    }

    fn reset_window_kernel(&mut self) {
        //let win_id = self.win_id;
        let subdivisions = self.subdivisions;
        self.run_main(Box::new(move |app| {
            for sliding_arc in &app.sliding_channels {
                let mut channel = sliding_arc.lock().unwrap();

                match &mut *channel {
                    SlidingImpl::DFT(dft) => {
                        // let wintype = match win_id {
                        //     0 => WindowType::BlackmanNutall,
                        //     1 => WindowType::ExpBlackman,
                        //     2 => WindowType::Rect,
                        //     _ => unreachable!(),
                        // };
                        dft.collector =
                            ChannelSWDFT::init_collector(subdivisions, dft.collector.windowtype);
                    }
                }
            }
        }));
    }

    // pub fn measurement_reset(&mut self) {
    //     for sliding_cell in &self.sliding_impls {
    //         let sliding_main = sliding_cell.borrow();
    //         let mut channel = sliding_main.sliding_rc.lock().unwrap();
    //         match &mut *channel {
    //             SlidingImpl::DFT(dft) => {
    //                 let measure_bin = dft.measure_bins.get_mut(0);
    //                 match measure_bin {
    //                     None => {}
    //                     Some(bin) => {
    //                         bin.reset_measurement(&self.zoom_config);
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    // pub fn measurement_change_freq(&mut self, left: bool) {
    //     for sliding_cell in &self.sliding_impls {
    //         let sliding_main = sliding_cell.borrow();
    //         let mut channel = sliding_main.sliding_rc.lock().unwrap();
    //         match &mut *channel {
    //             SlidingImpl::DFT(dft) => {
    //                 let measure_bin = dft.measure_bins.get_mut(0);
    //                 match measure_bin {
    //                     None => {}
    //                     Some(bin) => {
    //                         bin.adjust_freq_left(left, &self.zoom_config);
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    fn toggle_divisions_hz(&mut self) {
        self.divisions_hz = !self.divisions_hz;
    }

    pub fn on_mouse_wheel(&mut self, dy: f32) {
        self.zoom += dy;

        //let zoom = self.zoom / 10.0;
        let dy = dy / 10.0;

        let scaled_pos_x = self.mouse_pos.x / self.display_params.dx as f32;
        self.run_main(Box::new(move |app| {
            for sliding_arc in &app.sliding_channels {
                let mut channel = sliding_arc.lock().unwrap();
                match &mut *channel {
                    SlidingImpl::DFT(dft) => {
                        Self::on_mouse_wheel_config(&mut dft.config, dy, scaled_pos_x);
                    }
                }
            }
        }));

        self.reinit_spectrum();
    }

    pub fn on_mouse_wheel_config(zoom_config: &mut SpectrumConfig, dy: f32, scaled_pos_x: f32) {
        let mouse_freq = ChannelSWDFT::num_probe_x_to_freq(zoom_config, scaled_pos_x);

        {
            let octave_min = zoom_config.min_f.log2();
            let octave_max = zoom_config.max_f.log2();
            //println!(" pre: {} {}", octave_min, octave_max);

            //let octave_center = (octave_max + octave_min) / 2.0;

            let octave_center = mouse_freq.log2();
            //println!("octave_center: {}", octave_center);

            let delta_min = octave_center - octave_min;
            let delta_max = octave_max - octave_center;
            //println!("delta_octave1: {}", delta_octave);
            let scale = (0.5 as f32).powf(dy);
            let delta_min = delta_min * scale;
            let delta_max = delta_max * scale;
            //println!("delta_octave2: {}", delta_octave);

            let octave_min = octave_center - delta_min;
            let octave_max = octave_center + delta_max;

            //println!("post: {} {}", octave_min, octave_max);
            zoom_config.min_f = (2.0 as f32).powf(octave_min);
            zoom_config.max_f = (2.0 as f32).powf(octave_max);

            let f_nyquist = zoom_config.sample_rate as f32 / 2.0;
            if zoom_config.max_f > f_nyquist {
                zoom_config.max_f = f_nyquist;
            }
            if zoom_config.min_f < 4.0 {
                zoom_config.min_f = 4.0;
            }
        }
    }

    fn reinit_spectrum(&self) {
        self.run_main(Box::new(move |app| {
            for sliding_arc in &app.sliding_channels {
                let mut channel = sliding_arc.lock().unwrap();

                match &mut *channel {
                    SlidingImpl::DFT(dft) => {
                        //dft.spectrum_bins = ChannelSWDFT::make_spectrum(&zoom_config);
                        //ChannelSWDFT::init_spectrum(&self.zoom_config, &mut dft.spectrum_bins);
                        dft.reinit_my_spectrum();
                    }
                }
            }
        }));
    }

    pub fn on_cursor_move(&mut self, position: (f64, f64)) {
        let position_vec = cgmath::vec2(position.0 as f32, position.1 as f32);

        self.mouse_pos = position_vec;

        if self.dragging {
            let delta = position_vec - self.last_drag_pos;

            let threshold = self.display_params.dx as f32 / self._init_config.num_bins as f32;
            if delta.x.abs() >= threshold {
                let drag_bins = (delta.x / threshold) as i32;

                self.last_drag_pos.x += drag_bins as f32 * threshold;

                //println!("drag_bins: {}", drag_bins);
                if drag_bins > 0 {
                    self.move_spectrum(true, drag_bins);
                } else if drag_bins < 0 {
                    self.move_spectrum(false, -drag_bins);
                }
            }
        }
        //let l_channel = self.sliding_impl.lock();
        // let mut l_channel = self.sliding_impl.lock();
        // match &mut *l_channel {
        //     crate::sbswdft::SlidingImpl::DFT(dft) => {
        //let zoom_config = &mut self.zoom_config;

        //println!("frequency: {:.2} Hz", freq);
        // let _freq = ChannelSWDFT::num_probe_x_to_freq(
        //     zoom_config,
        //     self.mouse_pos.x / self.display_params.dx as f32,
        // );
        //drop(l_channel);
    }

    pub fn on_mouse_click(&mut self, pressed: bool, left: bool) {
        //println!("click, pressed:{} left:{}", pressed, left);
        if left {
            if pressed {
                let zoom_config = &mut self.zoom_config;
                let freq = ChannelSWDFT::num_probe_x_to_freq(
                    zoom_config,
                    self.mouse_pos.x / self.display_params.dx as f32,
                );
                println!("frequency: {:.2} Hz", freq);
            }
            self.dragging = pressed;
            self.last_drag_pos = self.mouse_pos;

            if pressed {
                if self.mouse_pos.y > self.display_params.dy as f32 - 30.0 {
                    self.toggle_divisions_hz();
                }
                if self.mouse_pos.x < 100.0 && self.mouse_pos.y < 50.0 {
                    self.toggle_divisions_hz();
                }
            }
        } else {
            // if pressed {
            //     let zoom_config = &mut self.zoom_config;
            //     let freq = ChannelSWDFT::num_probe_x_to_freq(
            //         zoom_config,
            //         self.mouse_pos.x / self.display_params.dx as f32,
            //     );
            //     println!("measuring frequency: {:.2} Hz", freq);

            //     let sliding_impl = &self.sliding_impls[0];
            //     let sliding_impl = sliding_impl.borrow();
            //     let mut l_channel = sliding_impl.sliding_rc.lock().unwrap();
            //     match &mut *l_channel {
            //         SlidingImpl::DFT(dft) => {
            //             dft.measure_bins.clear();
            //             let mut bin = MeasureBin::new();
            //             let c = &zoom_config;
            //             let sample_rate = c.sample_rate as f32;
            //             bin.bin.reinit_exact(
            //                 freq as f64,
            //                 phase_shift_per_sample_to_fixed_point(freq / sample_rate),
            //                 10 + c.wave_cycles_resolution as usize * 4
            //                     + ((c.wave_cycles_resolution * sample_rate)
            //                         / (c.resolution_low_f_shelf_hz + freq))
            //                         as usize,
            //                 true,
            //             );
            //             dft.measure_bins.push_back(bin);
            //         }
            //     }
        }
    }

    pub fn render(&mut self) -> (Vec<PosColVertex>, Vec<PosColTexVertex>) {
        let mut pc: Vec<PosColVertex> = Vec::new();
        let mut pct: Vec<PosColTexVertex> = Vec::new();

        self.render_grapher(&mut pc, &mut pct);
        (pc, pct)
    }

    fn make_divisions_grid(dx: f32, min_f: f32, max_f: f32) -> Vec<(f32, f32, i8)> {
        let mut v = Vec::new();

        // let n = 4;
        // let step = dx as f32 / n as f32;
        // let mut last_f = -1.0;
        // for i in 0..n {
        //     let x = (i as f32) * step;

        //     let f = ChannelSWDFT::exp_interpolate(min_f, max_f, x/dx);
        //     let m = 100.0;
        //     let f = (f / m).floor() * m;
        //     if f != last_f {
        //         last_f = f;

        //         let xx = ChannelSWDFT::exp_inverse(min_f, max_f, f);
        //         v.push((xx*dx, f));
        //     }
        // }
        //let bases = [1.0, 10.0, 100.0, 1000.0, 10000.0];

        let bases = [10000.0, 1000.0, 100.0, 10.0, 1.0];

        let mut added = BTreeSet::new();

        fn can_add(
            _v: &mut Vec<(f32, f32, i8)>,
            added: &mut BTreeSet<i32>,
            x_pos: f32,
            _f: f32,
        ) -> bool {
            let xi = x_pos as i32;
            if xi >= -10000 && xi < 10000 {
                added.range((xi - 70)..=(xi + 70)).next().is_none()
            } else {
                true
            }
        }

        for base in &bases {
            for i in 0..=9 {
                let f = base * i as f32;
                let f1 = base * (i + 1) as f32;
                if f1 > min_f && f < max_f {
                    {
                        let xx = dx * ChannelSWDFT::exp_inverse(min_f, max_f, f);
                        if can_add(&mut v, &mut added, xx, f) {
                            added.insert(xx as i32);
                            v.push((xx, f, 0));
                        }
                    }
                }
            }
        }

        // for base in &bases {
        //     for i in 0..9 {
        //         let f = base * i as f32;
        //         let f1 = base * (i + 1) as f32;
        //         if f1 > min_f || f < max_f {
        //             {
        //                 // halfs
        //                 let f2 = (f + f1) / 2.0;
        //                 let xx = dx * ChannelSWDFT::exp_inverse(min_f, max_f, f2);
        //                 if can_add(&mut v, &mut added, xx, f2) {
        //                     added.insert(xx as i32);
        //                     v.push((xx, f2));
        //                 }
        //             }
        //         }
        //     }
        // }

        fn try_add(
            depth: i32,
            min_f: f32,
            max_f: f32,
            dx: f32,
            v: &mut Vec<(f32, f32, i8)>,
            added: &mut BTreeSet<i32>,
            f: f32,
            base10: f32,
        ) {
            let can;
            // for ii in 1..=9 {
            //     let ff = f + ii as f32 * base10;

            //     {
            //         let xx = dx * ChannelSWDFT::exp_inverse(min_f, max_f, ff);
            //         if !can_add(v, added, xx, ff) {
            //             //if ii != 5 {
            //             can = false;
            //             //}
            //         }
            //     }
            // }
            {
                let ff1 = f + 9.0 * base10;
                let ff2 = f + 10.0 * base10;
                let xx1 = dx * ChannelSWDFT::exp_inverse(min_f, max_f, ff1);
                let xx2 = dx * ChannelSWDFT::exp_inverse(min_f, max_f, ff2);

                can = xx2 - xx1 > 70.0;
            }
            if can {
                let precision = -base10.log10().round() as i8;
                let precision = precision.max(0);
                for ii in 1..=9 {
                    let ff = f + ii as f32 * base10;
                    {
                        let xx = dx * ChannelSWDFT::exp_inverse(min_f, max_f, ff);
                        added.insert(xx as i32);
                        v.push((xx, ff, precision));
                    }
                }

                for ii in 0..=9 {
                    let ff = f + ii as f32 * base10;
                    let ff1 = f + (ii + 1) as f32 * base10;
                    if ff1 > min_f && ff < max_f {
                        if depth != 5 {
                            try_add(depth + 1, min_f, max_f, dx, v, added, ff, base10 / 10.0);
                        }
                    }
                }
            }
        }

        for base in &bases {
            for i in 0..=9 {
                let f = base * i as f32;
                let f1 = base * (i + 1) as f32;
                if f1 > min_f && f < max_f {
                    let base10 = base / 10.0;

                    try_add(0, min_f, max_f, dx, &mut v, &mut added, f, base10);
                }
            }
        }

        v
    }

    fn render_gui_status(&self, _pc: &mut Vec<PosColVertex>, pct: &mut Vec<PosColTexVertex>) {
        if self.divisions_hz {
            let mut fr = FontRenderer::new(self.font_atlas.clone(), pct);
            fr.ui_scale = self.gui_scale as f32;
            let offset = 10.0;

            let alive_threads;
            let wanted_threads;

            #[cfg(target_arch = "wasm32")]
            {
                use super::wasm_rayon::get_wasm_rayon_pool_builder;
                let wasm_rayon = get_wasm_rayon_pool_builder();
                if let Some(wasm_rayon) = wasm_rayon {
                    use std::sync::atomic::Ordering;
                    alive_threads = wasm_rayon.alive_threads.load(Ordering::Relaxed);
                    wanted_threads = wasm_rayon.num_threads;
                } else {
                    alive_threads = 0;
                    wanted_threads = 0;
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                alive_threads = 0;
                wanted_threads = 0;
            }

            let mem = super::kwasm::get_wasm_mem_size() as f64 / (1024.0 * 1024.0);
            let app_state = get_app_state();

            let state_string;
            let state_str = if app_state == AppState::Playing {
                ""
            } else {
                state_string = format!("state: {:?}", app_state);
                &state_string
            };
            fr.draw_string(
                format!(
                    "DFT Threads: {}/{}   mem: {:.2} MB   {}",
                    alive_threads, wanted_threads, mem, state_str
                )
                .as_str(),
                2.0,
                2.0,
                0xffaaffaa,
                false,
            );

            fr.draw_string(
                format!("[key: left/right/+/-/P/S]").as_str(),
                2.0,
                offset + 2.0,
                0xffaaffaa,
                false,
            );
        }
    }

    fn render_gui_divisions_grid(
        &self,
        snapshot: &StateSnapshot,
        _pc: &mut Vec<PosColVertex>,
        pct: &mut Vec<PosColTexVertex>,
        //dft: &mut ChannelSWDFT,
        gain: f64,
    ) {
        if self.divisions_hz {
            let mut fr = FontRenderer::new(self.font_atlas.clone(), pct);
            fr.ui_scale = self.gui_scale as f32;
            let offset = 10.0;
            fr.draw_string(
                format!(
                    "[up/down]        resolution: r = {:.2} wavelengths",
                    self.zoom_config.wave_cycles_resolution
                )
                .as_str(),
                2.0,
                offset + 12.0,
                0xffaaffaa,
                false,
            );

            fr.draw_string(
                format!(
                    " [M]                   method: {}",
                    snapshot.current_algo.describe()
                )
                .as_str(),
                2.0,
                offset + 22.0,
                0xffaaffaa,
                false,
            );

            // match &dft.spectrum_bins {
            //     SpectrumBins::DFT(_bins) => {
            if snapshot.current_algo == SpectrumBinsState::DFT {
                fr.draw_string(
                    format!(
                        " [O]                   window: {}",
                        match snapshot.window_type {
                            WindowType::Rect => "Rect",
                            WindowType::BlackmanNutall => "Blackman-Nuttall",
                            WindowType::ExpBlackman => "Exp * Blackman",
                            WindowType::LogNormal => "LogNormal",
                        }
                    )
                    .as_str(),
                    2.0,
                    offset + 32.0,
                    0xffaaffaa,
                    false,
                );
                if snapshot.window_type != WindowType::Rect {
                    fr.draw_string(
                        format!(" [B/N] window subdivisions: {}", snapshot.window_kernel_len)
                            .as_str(),
                        2.0,
                        offset + 42.0,
                        0xffaaffaa,
                        false,
                    );
                }
                //}
            }
            //     SpectrumBins::NC(_bins) => {}
            // }

            fr.draw_string(
                format!("[L]              log y scale: {}", self.logarithmic_scale,).as_str(),
                2.0,
                offset + 52.0,
                0xffaaffaa,
                false,
            );

            fr.draw_string(
                format!(
                    "[Z/X]            motion blur: {} Hz (every {} samples)",
                    snapshot.collect_frequency, snapshot.collect_every,
                )
                .as_str(),
                2.0,
                offset + 62.0,
                0xffaaffaa,
                false,
            );

            fr.draw_string(
                format!("gain: {:+.2} dB", 20.0 * gain.log10()).as_str(),
                2.0,
                offset + 82.0,
                0xffaaffaa,
                false,
            );

            let div_grid = Self::make_divisions_grid(
                self.display_params.gui_dx as f32,
                self.zoom_config.min_f,
                self.zoom_config.max_f,
            );

            //fr.ui_scale = 2.0;
            for div in div_grid {
                fr.draw_string(
                    format!("^ {:.1$} Hz", div.1, div.2 as usize).as_str(),
                    div.0 - 3.0,
                    self.display_params.gui_dy as f32 - 9.0,
                    0xffffffff,
                    false,
                );
            }
        }
    }

    fn push_rect_abcd(
        pc: &mut Vec<PosColVertex>,
        posa: [f32; 2],
        posb: [f32; 2],
        posc: [f32; 2],
        posd: [f32; 2],
        color: u32,
    ) {
        pc.push(PosColVertex {
            pos: [(posa[0] * 8.0) as i16, (posa[1] * 8.0) as i16],
            color: color,
        });
        pc.push(PosColVertex {
            pos: [(posb[0] * 8.0) as i16, (posb[1] * 8.0) as i16],
            color: color,
        });
        pc.push(PosColVertex {
            pos: [(posc[0] * 8.0) as i16, (posc[1] * 8.0) as i16],
            color: color,
        });
        pc.push(PosColVertex {
            pos: [(posd[0] * 8.0) as i16, (posd[1] * 8.0) as i16],
            color: color,
        });
    }

    fn push_line_ab(
        pc: &mut Vec<PosColVertex>,
        la: [f32; 2],
        lb: [f32; 2],
        lastpos: [[f32; 2]; 2],
        width: f32,
        color: u32,
    ) -> [[f32; 2]; 2] {
        let dx = la[0] - lb[0];
        let dy = la[1] - lb[1];
        let length1 = width / (0.0001 + (dx * dx + dy * dy).sqrt());

        let sx = -dy * length1;
        let sy = dx * length1;

        let posa = [lb[0] - sx, lb[1] - sy];
        let posb = [lb[0] + sx, lb[1] + sy];
        let mut posc = lastpos[0];
        let mut posd = lastpos[1];
        // first line
        if lastpos[0][1] == 0.0 {
            posc = [la[0] + sx, la[1] + sy];
            posd = [la[0] - sx, la[1] - sy];
        }

        Self::push_rect_abcd(pc, posa, posb, posc, posd, color);

        [posb, posa]
    }

    fn render_tooltip(
        &self,
        _pc: &mut Vec<PosColVertex>,
        pct: &mut Vec<PosColTexVertex>,
        dft: Option<&ChannelSWDFT>,
    ) {
        if self.dragging {
            let mut fr = FontRenderer::new(self.font_atlas.clone(), pct);
            fr.ui_scale = self.gui_scale as f32;

            let partial_x = self.mouse_pos.x / self.display_params.dx as f32;
            let f = ChannelSWDFT::exp_interpolate(
                self.zoom_config.min_f,
                self.zoom_config.max_f,
                partial_x,
            );
            fr.draw_string(
                format!("f: {:.1} Hz", f).as_str(),
                self.mouse_pos.x / (self.gui_scale as f32) - 30.0,
                self.mouse_pos.y / (self.gui_scale as f32) - 20.0,
                0xffaaffaa,
                false,
            );
            if let Some(dft) = dft {
                match &dft.spectrum_bins {
                    SpectrumBins::DFT(dft_bins) => {
                        let num_bin = partial_x * (dft_bins.len() as f32);
                        let bin = dft_bins.get(num_bin as usize);

                        match bin {
                            None => {}
                            Some(bin) => {
                                let window_len =
                                    bin.bin.length as f32 / self.zoom_config.sample_rate as f32;
                                let text = format!("window: {}", DisplayMsSecond(window_len));
                                fr.draw_string(
                                    text.as_str(),
                                    self.mouse_pos.x / (self.gui_scale as f32) - 100.0,
                                    self.mouse_pos.y / (self.gui_scale as f32) - 10.0,
                                    0xffaaffaa,
                                    false,
                                );
                            }
                        }
                    }
                    SpectrumBins::NC(nc_bins) => {
                        let num_bin = partial_x * (nc_bins.len() as f32);
                        let bin = nc_bins.get(num_bin as usize);

                        match bin {
                            None => {}
                            Some(bin) => {
                                let window_len =
                                    bin.bina.length as f32 / self.zoom_config.sample_rate as f32;
                                let text1 = format!("nc window: {}", DisplayMsSecond(window_len));
                                let delta_freq =
                                    self.zoom_config.sample_rate as f64 / (bin.bina.length as f64);
                                let text2 = format!("nc freq: {:.2} Hz", delta_freq);
                                fr.draw_string(
                                    text1.as_str(),
                                    self.mouse_pos.x / (self.gui_scale as f32) - 120.0,
                                    self.mouse_pos.y / (self.gui_scale as f32) - 10.0,
                                    0xffaaffaa,
                                    false,
                                );
                                fr.draw_string(
                                    text2.as_str(),
                                    self.mouse_pos.x / (self.gui_scale as f32) - 120.0,
                                    self.mouse_pos.y / (self.gui_scale as f32) - 0.0,
                                    0xffaaffaa,
                                    false,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_peaks(
        &self,
        pc: &mut Vec<PosColVertex>,
        spectrum: &Collected,
        gain: f64,
        alpha: u32,
    ) {
        let ybase = self.display_params.dy as f32 - 30.0;
        let len = spectrum.spectrum.len();
        let width_factor = (self.display_params.dx as f32) / (len as f32);

        if let Some(peaks) = &spectrum.peaks {
            for speak in peaks.iter() {
                let i = speak.probe_index;
                let mut val = speak.value;
                let s_probe_color = speak.color.rgba;

                val *= gain;
                val = self.maybe_log(val);

                if val < 0.0 {
                    val = 0.0;
                }
                let y = ybase - 0.004 * val as f32 * self.display_params.gui_dy as f32;

                let mut va = val * 6.0;

                if va > 256.0 {
                    va = 256.0;
                }
                va *= speak.alpha as f64;
                let a = va as u32;
                let mut r = s_probe_color & 0xFF;
                let mut g = (s_probe_color >> 8) & 0xFF;
                let mut b = (s_probe_color >> 16) & 0xFF;

                r *= a;
                g *= a;
                b *= a;

                r >>= 8;
                g >>= 8;
                b >>= 8;

                let darkened_color = r | (g << 8) | (b << 16);

                let color: u32 = (darkened_color & 0x00FFFFFF) | (alpha << 24);

                let w = val.sqrt() as f32;
                let ii = i as f32;
                let posa = [ii * width_factor - w, ybase];
                let posb = [ii * width_factor - w, y];
                let posc = [ii * width_factor + w, y];
                let posd = [ii * width_factor + w, ybase];

                Self::push_rect_abcd(pc, posa, posb, posc, posd, color);
            }
        }
    }

    fn render_lines(
        &self,
        pc: &mut Vec<PosColVertex>,
        spectrum: &Collected,
        gain: f64,
        alpha: u32,
    ) {
        let ybase = self.display_params.dy as f32 - 30.0;
        let len = spectrum.spectrum.len();
        let width_factor = (self.display_params.dx as f32) / (len as f32);

        let mut maxv = 0.00001;
        let mut last_y = 0.0;
        //let mut last_yy = 0.0;

        //debuga += alpha;
        let mut lastval: f64 = 0.0;
        let mut lastlinepos = [[0.0, 0.0], [0.0, 0.0]];

        for (i, ssample) in spectrum.spectrum.iter().enumerate() {
            let mut val = ssample.value;

            let s_probe_color = ssample.color.rgba;

            // for signal gain
            //val /= dft.rolling_gain * 2300.0;

            val *= gain;
            val = self.maybe_log(val);

            if val < 0.0 {
                val = 0.0;
            }
            if val > maxv {
                maxv = val;
            }
            //val = val.log10()*100.0+3.0;
            //val /= maxv;

            let max_of_vals = lastval.max(val);
            let avg_vals = (lastval + val) * 0.5;
            let y = ybase - 0.004 * avg_vals as f32 * self.display_params.gui_dy as f32;
            //let yy = ybase - 0.004 * max_of_vals as f32 * 1.12 * self.display_params.gui_dy as f32;

            if i != 0 {
                let mut va = max_of_vals * 6.0;

                if va > 256.0 {
                    va = 256.0;
                }
                let a = va as u32;
                let mut r = s_probe_color & 0xFF;
                let mut g = (s_probe_color >> 8) & 0xFF;
                let mut b = (s_probe_color >> 16) & 0xFF;

                r *= a;
                g *= a;
                b *= a;

                r >>= 8;
                g >>= 8;
                b >>= 8;

                let darkened_color = r | (g << 8) | (b << 16);

                let color: u32 = (darkened_color & 0x00FFFFFF) | (alpha << 24);

                let ii = i as f32;
                let iii = ii - 1.0;

                let la = [iii * width_factor, last_y];
                let lb = [ii * width_factor, y];

                // let posa = [ii * width_factor, y];
                // let posb = [ii * width_factor, yy];
                // let posc = [iii * width_factor, last_yy];
                // let posd = [iii * width_factor, last_y];

                // Self::push_rect_abcd(
                //     pc, posa, posb, posc, posd, color,
                // );
                let width = 0.5 + 0.03 * (max_of_vals as f32);

                lastlinepos = Self::push_line_ab(pc, la, lb, lastlinepos, width, color);
            }
            last_y = y;
            //last_yy = yy;
            lastval = val;
        }
    }

    fn maybe_log(&self, val: f64) -> f64 {
        // magic number for some offset
        if self.logarithmic_scale {
            (val * 0.08 + 0.000000001).log10() * 280.0
        } else {
            val
        }
    }

    fn render_fills(
        &self,
        pc: &mut Vec<PosColVertex>,
        spectrum: &Collected,
        gain: f64,
        alpha: u32,
    ) {
        let ybase = self.display_params.dy as f32 - 30.0;
        let len = spectrum.spectrum.len();
        let width_factor = (self.display_params.dx as f32) / (len as f32);

        let mut maxv = 0.00001;
        let mut last_y = 0.0;

        //debuga += alpha;
        let mut lastval: f64 = 0.0;

        for (i, ssample) in spectrum.spectrum.iter().enumerate() {
            let mut val = ssample.value;
            val *= gain;
            val = self.maybe_log(val);
            let s_probe_color = ssample.color.rgba;

            // for signal gain
            //val /= dft.rolling_gain * 2300.0;

            if val < 0.0 {
                val = 0.0;
            }
            if val > maxv {
                maxv = val;
            }
            //val = val.log10()*100.0+3.0;
            //val /= maxv;

            let max_of_vals = lastval.max(val);
            let y = ybase - 0.004 * val as f32 * self.display_params.gui_dy as f32;
            if i != 0 {
                let mut va = max_of_vals * 6.0;

                if va > 256.0 {
                    va = 256.0;
                }
                let a = va as u32;
                let mut r = s_probe_color & 0xFF;
                let mut g = (s_probe_color >> 8) & 0xFF;
                let mut b = (s_probe_color >> 16) & 0xFF;

                r *= a;
                g *= a;
                b *= a;

                r >>= 8;
                g >>= 8;
                b >>= 8;

                let darkened_color = r | (g << 8) | (b << 16);

                let color: u32 = (darkened_color & 0x00FFFFFF) | (alpha << 24);

                let ii = i as f32;
                let iii = ii - 1.0;

                //let la = [iii * width_factor, last_y];
                //let lb = [ii * width_factor, y];

                let posa = [ii * width_factor, ybase];
                let posb = [ii * width_factor, y];
                let posc = [iii * width_factor, last_y];
                let posd = [iii * width_factor, ybase];

                // Self::push_rect_abcd(
                //     pc, posa, posb, posc, posd, color,
                // );
                //let width = 0.5 + 0.03 * (lastval as f32);

                //lastlinepos = Self::push_line_ab(pc, la, lb, lastlinepos, width, color);

                Self::push_rect_abcd(pc, posa, posb, posc, posd, color);
            }
            lastval = val;
            last_y = y;
        }
    }

    pub fn render_grapher(
        &mut self,
        //channels: &mut Vec<Arc<Mutex<SlidingImpl>>>,
        pc: &mut Vec<PosColVertex>,
        pct: &mut Vec<PosColTexVertex>,
    ) {
        self.render_gui_status(pc, pct);
        if false {
            let display = &self.display_params;
            let posa = [0.0, 0.0];
            let posb = [0.0, display.gui_dy as f32];
            let posc = [display.gui_dx as f32 / 2.0, display.gui_dy as f32];
            let posd = [display.gui_dx as f32 / 2.0, 0.0];

            let texa = [0.0, 0.0];
            let texb = [0.0, 1.0];
            let texc = [1.0, 1.0];
            let texd = [1.0, 0.0];

            let color = 0xFFFFFFFF;
            pct.push(PosColTexVertex {
                pos: posa,
                color: color,
                tex: texa,
            });
            pct.push(PosColTexVertex {
                pos: posb,
                color: color,
                tex: texb,
            });
            pct.push(PosColTexVertex {
                pos: posd,
                color: color,
                tex: texd,
            });
            pct.push(PosColTexVertex {
                pos: posd,
                color: color,
                tex: texd,
            });
            pct.push(PosColTexVertex {
                pos: posb,
                color: color,
                tex: texb,
            });
            pct.push(PosColTexVertex {
                pos: posc,
                color: color,
                tex: texc,
            });
        }

        //let gl = &self.display.gl;
        for (channel_num, sliding_cell) in self.sliding_impls.iter().enumerate() {
            let mut sliding_main = sliding_cell.borrow_mut();

            loop {
                let msg = sliding_main.spectrum_receiver.try_recv();
                match msg {
                    Ok(RendererMsg::NewSpectrum(collected)) => {
                        sliding_main.last_rolling_gain = collected.cur_rolling_gain;
                        sliding_main.collected_spectrums.push_front(collected);
                    }
                    Ok(RendererMsg::ConfigUpdate(config)) => {
                        self.zoom_config = config;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }

            let mut wanted = 20;
            if let Some(first) = sliding_main.collected_spectrums.front() {
                wanted = 3 * first.snapshot.collect_frequency / 60;
            }

            let mut num_collected = sliding_main.collected_spectrums.len();
            if num_collected > wanted {
                num_collected = wanted;
            }
            while sliding_main.collected_spectrums.len() > wanted {
                sliding_main.collected_spectrums.pop_back();
            }
            //let mut l_channel = sliding_main.sliding_rc.lock().unwrap();
            // match &mut *l_channel {
            //     SlidingImpl::DFT(dft) => {
            let gain = 1.0 / (sliding_main.last_rolling_gain * 1.3);

            if channel_num == 0 {
                if let Some(first) = sliding_main.collected_spectrums.front() {
                    self.render_gui_divisions_grid(&first.snapshot, pc, pct, gain);
                }

                //self.render_measurement(pc, pct, dft);
                self.render_tooltip(pc, pct, None);
            }

            // let wanted = (1 * dft.collect_frequency) / 140;
            // let mut num_collected = dft.collected_spectrums.len();
            // if num_collected > wanted {
            //     num_collected = wanted;
            // }
            // while dft.collected_spectrums.len() > wanted {
            //     dft.collected_spectrums.pop_back();
            // }

            // precise with 16 bits++
            let alphap = if num_collected == 0 {
                0xFFFFFF
            } else {
                0xFFFFFF / num_collected as u32
            };
            //let alpha = 10;
            let mut lastalpha = 0;
            let mut currentalpha;
            let mut currentalphap = 0;
            //let _debuga = 0;

            //let collected: Vec<Arc<Mutex<Collected>>> =
            //    dft.collected_spectrums.iter().map(|c| c.clone()).collect();
            let collected = &mut sliding_main.collected_spectrums;

            // free the mutex
            //drop(dft);
            //drop(l_channel);

            for collected_index in 0..num_collected {
                currentalphap += alphap;
                currentalpha = currentalphap >> 16;

                let alpha = currentalpha - lastalpha;

                let opt_spectrum = collected.get_mut(collected_index);
                match opt_spectrum {
                    None => break,
                    Some(mut spectrum) => {
                        //let width_factor = 1.0 / (len as f32);
                        let mut collected: &mut Collected = &mut spectrum;

                        match &collected.rendered {
                            Some(rendered) => {
                                pc.extend_from_slice(&rendered);
                            }
                            None => {
                                let mut lpc = Vec::new();
                                match self.graph_type {
                                    GraphType::Peaks => {
                                        self.render_peaks(&mut lpc, collected, gain, alpha);
                                    }
                                    GraphType::Line => {
                                        self.render_lines(&mut lpc, collected, gain, alpha);
                                    }
                                    GraphType::Fill => {
                                        self.render_fills(&mut lpc, collected, gain, alpha);
                                    }
                                }
                                pc.extend_from_slice(&lpc);
                                collected.rendered = Some(lpc);
                            }
                        }
                    }
                }
                lastalpha = currentalpha;
            }
            /*
            fr.draw_string(
                &format!("alpha sum: {}", debuga).as_str(),
                300.0,
                200.0,
                0xffffffff,
                true,
            );*/
            //     }
            //     _ => {}
            // }
        }
    }
}

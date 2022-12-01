extern crate lazy_static;

use crate::spectrumapp::spectrumui::StateSnapshot;

use super::PosColVertex;
use lazy_static::lazy_static;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Mutex;
//use parking_lot::Mutex;
use std::sync::Arc;

// single bin sliding window DFT

pub enum SlidingImpl {
    DFT(ChannelSWDFT),
    //Correlator(ChannelCorrs),
}

use std::collections::VecDeque;

use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;

pub fn positive_mod(a: i32, b: i32) -> usize {
    let x = a % b;
    if x < 0 {
        (x + b) as _
    } else {
        x as _
    }
}

pub static QUANTIZER_LEVELS_POW: i64 = 15;
pub static QUANTIZER_LEVELS: i64 = 32767;
pub static QUANTIZER_LEVELS_F: f32 = QUANTIZER_LEVELS as f32;
pub static QUANTIZER_LEVELS_F64: f64 = QUANTIZER_LEVELS as f64;
pub static QUANTIZER_LEVELS_INV_F: f32 = 1.0 / (QUANTIZER_LEVELS as f32);
pub static QUANTIZER_LEVELS_INV_F64: f64 = 1.0 / (QUANTIZER_LEVELS as f64);

pub const TAU: f32 = 6.28318530717958647692528676655900577_f32;

#[derive(Default, Clone)]
pub struct ComplexI32 {
    pub re: i32,
    pub im: i32,
}
#[derive(PartialEq, Default, Clone, Copy)]
pub struct ComplexI64 {
    pub re: i64,
    pub im: i64,
}
impl ComplexI64 {
    pub fn magnitude_squared(&self) -> i64 {
        self.re * self.re + self.im * self.im
    }

    pub fn rotate(&self, angle: u32) -> Self {
        let re = self.re;
        let im = self.im;

        let c = fixedp_cos(angle) as i64;
        let s = fixedp_sin(angle) as i64;

        Self {
            re: (re * c - im * s) / (QUANTIZER_LEVELS + 1),
            im: (re * s + im * c) / (QUANTIZER_LEVELS + 1),
        }
    }

    pub fn to_f64(&self) -> ComplexF64 {
        let mut val_real = self.re as f64;
        val_real *= QUANTIZER_LEVELS_INV_F64;
        //val_real *= overflow_correction as f64;
        //val_real /= 0xFFF0 as f64;

        let mut val_imag = self.im as f64;
        val_imag *= QUANTIZER_LEVELS_INV_F64;
        //val_imag *= overflow_correction as f64;
        //val_imag /= 0xFFF0 as f64;

        ComplexF64 {
            re: val_real,
            im: val_imag,
        }
    }
}

impl std::ops::Add for ComplexI64 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        ComplexI64 {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }
}

impl std::fmt::Debug for ComplexI64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}+{}i", self.re, self.im)
    }
}

#[derive(PartialEq, Default, Clone, Debug, Copy)]
pub struct ComplexF64 {
    pub re: f64,
    pub im: f64,
}
impl ComplexF64 {
    pub fn magnitude_squared(&self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    pub fn rotate(&self, angle: f64) -> Self {
        let re = self.re;
        let im = self.im;

        let c = angle.cos();
        let s = angle.sin();

        Self {
            re: re * c - im * s,
            im: re * s + im * c,
        }
    }
}

const SINGLESUM: bool = false;

pub fn decr_modulo(pos: &mut usize, len: usize) {
    *pos = if *pos == len - 1 {
        0
    } else {
        pos.wrapping_add(1)
    };
}

fn hsv2rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let hp = h / 60.0;
    let c = v * s;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());

    let m = v - c;
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    match hp as i32 {
        0 => {
            r = c;
            g = x;
        }
        1 => {
            r = x;
            g = c;
        }
        2 => {
            g = c;
            b = x;
        }
        3 => {
            g = x;
            b = c;
        }
        4 => {
            r = x;
            b = c;
        }
        5 => {
            r = c;
            b = x;
        }
        _ => {}
    };

    (m + r, m + g, m + b)
}

fn note2colorchord(note: f32) -> f32 {
    let note = 12.0 * (note % 1.0);

    if note < 4.0 {
        //Needs to be YELLOW->RED
        (4.0 - note) / 24.0
    } else if note < 8.0 {
        //            [4]  [8]
        //Needs to be RED->BLUE
        (4.0 - note) / 12.0
    } else {
        //             [8] [12]
        //Needs to be BLUE->YELLOW
        (12.0 - note) / 8.0 + 1.0 / 6.0
    }
}

pub fn hz2color_rgb(freq_hz: f32) -> (f32, f32, f32) {
    let key;
    //let reverse; // = 1.0;
    key = 440.0; // regular
                 //key = 280.0; // lumi video keyboard
                 //key = 275.0; // color chord default
    //reverse = -1.0;

    // mapping to yellow (2^(1/12))
    let key = key; // * 1.12246204830;

    let octave = 10.0 + (0.0001 + freq_hz as f64 / key).log2();

    //let angle = (360.0 * 100.0 + reverse * octave * 360.0) % 360.0;

    let angle = 360.0 * note2colorchord(octave as f32);
    let angle = (angle + 360.0) % 360.0;

    hsv2rgb(angle as f32, 1.0, 1.0)
}

pub fn hz2color(freq_hz: f32) -> u32 {
    let (mut r, mut g, mut b) = hz2color_rgb(freq_hz);

    r *= 255.0;
    g *= 255.0;
    b *= 255.0;

    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | 0xFF << 24
}

// #[test]
// fn print_hz2color440() {
//     let step = (2.0_f64).powf(1.0 / 12.0);
//     let mut f = 440.0_f64;
//     for _ in 0..13 {
//         let (r, g, b) = hz2color_rgb(f as f32);
//         let hue = (f / 440.0).log2() * 360.0;

//         println!("{:.2}", f);
//         //println!("{:3.0} {{\\color[rgb]{{{:.2},{:.2},{:.2}}}\\textbf{{kolor}}}}", hue, r, g, b);

//         //let

//         f = f * step;
//     }
// }

fn round_next_power_of_2(mut v: u32) -> u32 {
    // compute the next highest power of 2 of 32-bit v

    v -= 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v += 1;
    v
}

pub struct DftBin {
    pub pulsation: u32,
    // fixed-point, 32768 is 2*pi
    pub current_phase: u32,
    pub length: usize,
    pub lengthf: f64,
    pub inv_lengthf: f64,
    pub current_length: usize,
    pub state_sum_real: i64,
    pub state_sum_imag: i64,
    // pub partial_samples: Vec<ComplexI32>,
    // pub partial_samples_pos: usize,
    pub partial_sums: Vec<ComplexI64>,
    pub partial_sums_pos: usize,
}

//#[derive(Clone)]

// TODO

#[derive(Clone)]
pub struct BinMeta {
    pub color: SColor,
    pub octave: f32,
    pub a_weight: f64,
    pub samplerate_octave: usize,
}

impl BinMeta {
    pub fn new() -> Self {
        Self {
            color: SColor::new(0xFFFFFFFF),
            octave: 1.0,
            a_weight: 1.0,
            samplerate_octave: 0,
        }
    }

    pub fn reinit(&mut self, freq_hz: f64, samplerate_hz: u32) {
        //println!("reinit: {} {}", freq_hz, samplerate_hz);
        self.color = SColor::new(hz2color(freq_hz as f32));
        self.octave = 10.0 + (0.001 + freq_hz as f32 / 440.0).log2();
        //println!("octave: {}", self.octave);
        if self.octave.is_nan() {
            self.octave = 0.0;
        }
        self.a_weight = a_weighting(freq_hz as f32);

        self.samplerate_octave = needed_samplerate_octave(freq_hz, samplerate_hz as f64);
    }
}

pub fn needed_samplerate_octave(freq_hz: f64, samplerate_hz: f64) -> usize {
    // add a little more for some margin

    // TODO: do the same with log2
    let f = freq_hz * 2.1;
    let mut sr = samplerate_hz;
    let mut octave = 0;

    for _ in 0..20 {
        if f < sr {
            octave += 1;
            sr *= 0.5;
        } else {
            break;
        }
    }
    octave
}

#[test]
pub fn test_needed_sr() {
    let mut f = 20.0;
    let sr = 24000.0;

    for _ in 0..16 {
        println!("for {} {} it's {}", f, sr, needed_samplerate_octave(f, sr));
        f = f * 2.0;
    }
}

pub struct RegularBin {
    pub meta: BinMeta,
    pub bin: DftBin,
}

impl RegularBin {
    pub fn new() -> Self {
        Self {
            meta: BinMeta::new(),
            bin: DftBin::new(),
        }
    }
}

pub struct NCBin {
    pub meta: BinMeta,
    pub bina: DftBin,
    pub binb: DftBin,
}

impl NCBin {
    pub fn new() -> Self {
        Self {
            meta: BinMeta::new(),
            bina: DftBin::new(),
            binb: DftBin::new(),
        }
    }
}

//#[derive(Clone)]
pub enum SpectrumBins {
    DFT(VecDeque<RegularBin>),
    NC(VecDeque<NCBin>),
}
impl SpectrumBins {
    pub fn state(&self) -> SpectrumBinsState {
        match self {
            SpectrumBins::DFT(_) => SpectrumBinsState::DFT,
            SpectrumBins::NC(_) => SpectrumBinsState::NC,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum SpectrumBinsState {
    DFT,
    NC,
}

impl SpectrumBinsState {
    pub fn describe(&self) -> &'static str {
        match self {
            SpectrumBinsState::DFT => "Regular DFT",
            SpectrumBinsState::NC => "NC",
        }
    }
}
// impl SpectrumBin {
//     fn new() -> Self {
//         SpectrumBin::DFT(DftBin::new())
//     }

//     pub fn reinit(&mut self, c: &SpectrumConfig, freq: f64, empty: bool) {
//         match self {
//             SpectrumBin::DFT(dftbin) => {
//                 reinit_dft_by_config(c, dftbin, freq, empty);
//             }
//             SpectrumBin::NC(_) => {

//             }
//         }
//     }

// }

// impl Clone for DftBin {
//     fn clone(&self) -> Self {
//         Self {
//             pulsation: self.pulsation,
//             current_phase: 0,
//             length: self.length,
//             lengthf: self.lengthf,
//             inv_lengthf: 1.0 / self.lengthf,
//             current_length: 0,
//             state_sum_real: 0,
//             state_sum_imag: 0,
//             // partial_samples: Vec::new(),
//             // partial_samples_pos: 0,
//             partial_sums: Vec::new(),
//             partial_sums_pos: 0,
//             color: 0xFFFFFFFF,
//             octave: 1.0,
//             a_weight: self.a_weight,
//         }
//     }
// }

impl DftBin {
    pub fn new() -> Self {
        Self {
            pulsation: 0,
            current_phase: 0,
            length: 0,
            lengthf: 1.0,
            inv_lengthf: 1.0,
            current_length: 0,
            state_sum_real: 0,
            state_sum_imag: 0,
            // partial_samples: vec![Default::default(); length],
            // partial_samples_pos: 0,
            //partial_sums: vec![unsafe { std::mem::zeroed() }; length],
            partial_sums: vec![Default::default(); 0],
            partial_sums_pos: 0,
        }
    }
    pub fn frequency(&self, sample_rate: f32) -> f32 {
        // inverse phase_shift_per_sample_to_fixed_point(freq / sample_rate)

        fixed_point_to_phase_shift_per_sample(self.pulsation) * sample_rate
    }
    pub fn frequency64(&self, sample_rate: f64) -> f64 {
        fixed_point_to_phase_shift_per_sample64(self.pulsation) * sample_rate
    }

    // pub fn reinit(&mut self, c: &SpectrumConfig, freq: f64, empty: bool) {
    //     match self {
    //         SpectrumBin::DFT(dftbin) => {
    //             reinit_dft_by_config(c, dftbin, freq, empty);
    //         }
    //         SpectrumBin::NC(_) => {

    //         }
    //     }
    // }
    pub fn window_size(resolution: f32, sample_rate: f32, freq: f32, low_f_shelf_hz: f32) -> usize {
        let h = 20 + (resolution * 4.0) as usize;
        let l = low_f_shelf_hz;
        let w = h + ((resolution * sample_rate) / (l + freq)) as usize;

        w
    }

    pub fn reinit_by_config(&mut self, c: &SpectrumConfig, freq: f64, empty: bool) {
        let sample_rate = c.sample_rate as f64;
        self.reinit_exact(
            freq,
            phase_shift_per_sample_to_fixed_point64(freq / sample_rate),
            Self::window_size(
                c.wave_cycles_resolution,
                sample_rate as f32,
                freq as f32,
                c.resolution_low_f_shelf_hz,
            ),
            empty,
        );
    }

    pub fn reinit_exact(&mut self, freq: f64, pulsation: u32, new_length: usize, empty: bool) {
        let _freq = freq.abs();
        //self.partial_sums.resize(0, Default::default());

        // if self.partial_sums.capacity() >= 8 * length {
        //     // prevent too huge allocs
        //     self.partial_sums = vec![Default::default(); length];
        // } else {
        //     self.partial_sums.resize(length, Default::default());
        // }
        let mut smooth = false;
        let new_capacity = round_next_power_of_2(new_length as u32) as usize;
        if new_capacity == self.partial_sums.capacity() {
            // very fast, zero alloc
            smooth = true;
            self.current_length = new_length.min(self.current_length);
        } else {
            let mut vec = Vec::with_capacity(new_capacity);
            if !empty {
                if new_length <= self.length {
                    let pos = (self.partial_sums_pos + 1) % self.partial_sums.len();

                    let mut a = &self.partial_sums[pos..self.partial_sums.len()];
                    let mut b = &self.partial_sums[0..=self.partial_sums_pos];
                    let mut cap = new_length;

                    if b.len() > cap {
                        b = &b[b.len() - cap..];
                    }
                    cap -= b.len();
                    if a.len() > cap {
                        a = &a[a.len() - cap..];
                    }
                    vec.extend_from_slice(a);
                    vec.extend_from_slice(b);

                    smooth = true;
                    self.partial_sums_pos = new_length - 1;
                } else if self.partial_sums.len() > 0 {
                    let pos = (self.partial_sums_pos + 1) % self.partial_sums.len();

                    let a = &self.partial_sums[pos..self.partial_sums.len()];
                    let b = &self.partial_sums[0..=self.partial_sums_pos];
                    vec.extend_from_slice(a);
                    vec.extend_from_slice(b);

                    smooth = true;
                    self.partial_sums_pos = self.partial_sums.len() - 1;
                }
                self.current_length = new_length.min(self.current_length);
            }
            vec.resize(new_capacity, Default::default());
            self.partial_sums = vec;
        }

        self.length = new_length;
        self.lengthf = new_length as f64;
        self.inv_lengthf = 1.0 / self.lengthf;

        self.pulsation = pulsation;
        //if pulsation != self.pulsation {
        // self.color = hz2color(freq as f32);
        // self.octave = 10.0 + (0.001 + freq as f32 / 440.0).log2();
        // if self.octave.is_nan() {
        //     self.octave = 0.0;
        // }
        // self.a_weight = a_weighting(freq as f32);
        if !smooth {
            self.reset();
        }
    }
    pub fn reset(&mut self) {
        self.state_sum_real = 0;
        self.state_sum_imag = 0;
        self.partial_sums_pos = self.partial_sums_pos % self.length;
    }

    pub fn magnitude_squared_all(&self) -> f64 {
        let mut val_real = (self.state_sum_real / (self.length as i64)) as f64;
        val_real *= QUANTIZER_LEVELS_INV_F64;

        let mut val_imag = (self.state_sum_imag / (self.length as i64)) as f64;
        val_imag *= QUANTIZER_LEVELS_INV_F64;

        val_real * val_real + val_imag * val_imag
    }

    #[inline]
    pub fn sum_ranged_from_to(&self, from: usize, to: usize) -> ComplexI64 {
        let pos = self.partial_sums_pos as i32;
        let partial_sums = &self.partial_sums;
        let l = partial_sums.len() as i32;
        let mut ai = pos.wrapping_sub(from as i32);
        let mut bi = pos.wrapping_sub(to as i32);
        if ai < 0 {
            ai = ai.wrapping_add(l);
        }
        if bi < 0 {
            bi = bi.wrapping_add(l);
        }

        let a;
        let b;

        #[cfg(debug_assertions)]
        {
            a = partial_sums.get(ai as usize).unwrap();
            b = partial_sums.get(bi as usize).unwrap();
        }

        #[cfg(not(debug_assertions))]
        unsafe {
            a = partial_sums.get_unchecked(ai as usize);
            b = partial_sums.get_unchecked(bi as usize);
        }

        ComplexI64 {
            re: a.re.wrapping_sub(b.re),
            im: a.im.wrapping_sub(b.im),
        }
    }

    pub fn sum_ranged_all(&self) -> ComplexI64 {
        let window_max_index = self.current_length - 1;
        let range_sum = self.sum_ranged_from_to(0, window_max_index);
        range_sum
    }

    pub fn sum_complex_kerneled(&self, kernel: &[i64]) -> ComplexI64 {
        //kernel = &[0x3EEE, 0x3EEE, 0x3EEE, 0x3EEE];
        let kernel_len = kernel.len();
        //let kernel_lenf = kernel.len() as f32;
        let window_max_index = self.current_length - 1; //self.partial_sums.len() - 1;
        let mut sum = ComplexI64 { re: 0, im: 0 };

        //let mut steps = Vec::new();
        //steps.push(0);

        if true {
            let mut from = 0;
            let mut i = 1;
            let step_dither = 0x10000;
            let step = (window_max_index * step_dither) / (kernel_len);

            //let mut dsum = ComplexI64 { re: 0, im: 0 };
            let mut next;
            for factor in kernel.iter() {
                next = (i * step) / step_dither;
                i += 1;
                //steps.push(next);

                let range = &self.sum_ranged_from_to(from, next);
                let window_factor = *factor as i64;
                sum.re = sum.re.wrapping_add(
                    range
                        .re
                        //.wrapping_shr(overflow_correction_shift)
                        .wrapping_mul(window_factor),
                );
                sum.im = sum.im.wrapping_add(
                    range
                        .im
                        //.wrapping_shr(overflow_correction_shift)
                        .wrapping_mul(window_factor),
                );

                //dsum.re += range.re;
                //dsum.im += range.im;

                from = next;
            }
            //assert!(next >= 0 && next <= window_max_index);
            //let mut check = self.sum_ranged_from_to(0, window_max_index);
            //assert_eq!(dsum, check);
            //sum = check;
        }

        //crate::console_log!("{:?}", steps);
        //panic!("...");
        //sum.re /= 0x10000;
        //sum.im /= 0x10000;

        sum
    }

    pub fn sum_magnitude_kerneled(&self, kernel: &[i64]) -> (f64, ComplexF64) {
        let sum = self.sum_complex_kerneled(kernel);

        //let overflow_correction_shift = 0;
        //let overflow_correction = 1 << overflow_correction_shift;

        let mut val = sum.to_f64();
        let scale = 1.0 / (self.lengthf * (0xFFF0 as f64));
        val.re *= scale;
        val.im *= scale;

        let magnitude_squared = val.re * val.re + val.im * val.im;
        (magnitude_squared, val)
    }

    pub fn magnitude_squared_from_to(&self, from: usize, to: usize) -> f64 {
        let sum = self.sum_ranged_from_to(from, to);
        let mut val_real = (sum.re / (self.length as i64)) as f64;
        val_real *= QUANTIZER_LEVELS_INV_F64;

        let mut val_imag = (sum.im / (self.length as i64)) as f64;
        val_imag *= QUANTIZER_LEVELS_INV_F64;

        val_real * val_real + val_imag * val_imag
    }

    pub fn magnitude_all(&self) -> f64 {
        self.magnitude_squared_all().sqrt()
    }

    pub fn advance(&mut self, n_new: usize, samples: &[i32], ring_offset: usize) {
        //let len = samples.len();
        let ring_mask = samples.len() - 1;

        //let off = self.offset;
        let mut sum_real = self.state_sum_real;
        let mut sum_imag = self.state_sum_imag;

        let mut current_phase = self.current_phase;
        let pulsation = self.pulsation;
        let partial_sums = &mut self.partial_sums;
        let mut partial_sums_pos = self.partial_sums_pos;
        for i in 0..n_new {
            let s1 = samples[((ring_offset + 1).wrapping_add(i).wrapping_sub(n_new)) & ring_mask];

            let current_phaseu = fp_undither(current_phase);
            let s2real = fixedp_cos(current_phaseu);
            let s2imag = fixedp_sin(current_phaseu);

            let s1q = s1;
            let partial_real = s1q.wrapping_mul(s2real as i32);
            let partial_imag = s1q.wrapping_mul(s2imag as i32);

            // 64 - (16+16+12) 44 = 22 bit free
            sum_real += partial_real as i64;
            sum_imag += partial_imag as i64;

            if !SINGLESUM {
                let l = partial_sums.len();
                decr_modulo(&mut partial_sums_pos, l);
                let c = ComplexI64 {
                    re: sum_real,
                    im: sum_imag,
                };

                #[cfg(debug_assertions)]
                {
                    partial_sums[partial_sums_pos] = c;
                }

                #[cfg(not(debug_assertions))]
                unsafe {
                    *(partial_sums.get_unchecked_mut(partial_sums_pos)) = c;
                }
            }
            current_phase += pulsation;
        }
        self.partial_sums_pos = partial_sums_pos;

        self.current_phase = current_phase & (FIXED_POINT_FRACTIONAL_DITHER_MASK as u32);

        let plen = self.length;

        self.current_length += n_new;
        if self.current_length > plen {
            self.current_length = plen;
        }
        self.state_sum_real = sum_real;
        self.state_sum_imag = sum_imag;
    }
}

pub struct MeasureBin {
    pub bin: DftBin,
    pub cumulative_phase_change: f64,
    pub last_phase: f64,
    pub sample_counter: i64,
    pub first_measurement: bool,
}

impl MeasureBin {
    pub fn new() -> Self {
        Self {
            bin: DftBin::new(),
            cumulative_phase_change: 0.0,
            last_phase: 0.0,
            sample_counter: 0,
            first_measurement: true,
        }
    }

    pub fn reset_measurement(&mut self, _c: &SpectrumConfig) {
        self.cumulative_phase_change = 0.0;
        self.sample_counter = 0;
        self.first_measurement = true;
    }

    pub fn auto_adjust(&mut self, c: &SpectrumConfig) {
        let sample_rate = c.sample_rate as f32;
        let old_freq = self.bin.frequency(sample_rate);

        let phase_change_speed = self.cumulative_phase_change / (self.sample_counter as f64);
        let phase_change_speed = phase_change_speed / (std::f64::consts::PI * 2.0);
        let phase_change_speed = sample_rate * (phase_change_speed as f32);

        println!("adjusting: {:.6} Hz", phase_change_speed);
        println!("   period: {:.6} s", 1.0 / phase_change_speed);

        let freq = old_freq + phase_change_speed as f32;

        self.adjust(freq as f64, c);
    }
    pub fn adjust_freq_left(&mut self, left: bool, c: &SpectrumConfig) {
        let delta = 0.0001;
        let change = if left { -delta } else { delta };

        let sample_rate = c.sample_rate as f64;
        let old_freq = self.bin.frequency64(sample_rate);
        let freq = old_freq + change;

        println!("adjusting o: {:.6} Hz", old_freq);
        println!("adjusting n: {:.6} Hz", freq);
        println!(
            "adjusting p: {}",
            phase_shift_per_sample_to_fixed_point64(freq / sample_rate)
        );
        //println!("   period: {:.6} s", 1.0 / freq);

        self.adjust(freq, c);
    }

    pub fn adjust(&mut self, freq: f64, c: &SpectrumConfig) {
        let sample_rate = c.sample_rate as f64;
        //let old_freq = self.bin.frequency(sample_rate);

        let res = c.wave_cycles_resolution;
        let shelf = c.resolution_low_f_shelf_hz;
        self.bin.reinit_exact(
            freq,
            phase_shift_per_sample_to_fixed_point64(freq / sample_rate),
            10 + res as usize * 4 + ((res * sample_rate as f32) / (shelf + freq as f32)) as usize,
            true,
        );
        self.cumulative_phase_change = 0.0;
        self.sample_counter = 0;
        self.first_measurement = true;
    }
}

// dither is required for low frequencies where pulsation is like 5, 4, 3 or less per sample
// it fixes stairs in the spectrogram
const PHASE_DITHER_POW: usize = 12;
const PHASE_DITHER: usize = 1 << PHASE_DITHER_POW;
const FIXED_POINT_FRACTIONAL: usize = 1024 * 32;
const FIXED_POINT_FRACTIONAL_MASK: usize = FIXED_POINT_FRACTIONAL - 1;
const FIXED_POINT_FRACTIONAL_DITHER: usize = FIXED_POINT_FRACTIONAL * PHASE_DITHER;
const FIXED_POINT_FRACTIONAL_DITHER_MASK: usize = FIXED_POINT_FRACTIONAL_DITHER - 1;
const FIXED_POINT_PHASE_MULTIPLIER: f64 =
    1.0 / (PHASE_DITHER as f64 * FIXED_POINT_FRACTIONAL as f64);
const FIXED_POINT_PHASE_MULTIPLIER_TAU: f64 =
    std::f64::consts::TAU / (PHASE_DITHER as f64 * FIXED_POINT_FRACTIONAL as f64);

const FIXED_POINT_90DEG: usize = FIXED_POINT_FRACTIONAL / 4;
static mut SINCOS: [i16; FIXED_POINT_FRACTIONAL] = [0; FIXED_POINT_FRACTIONAL];

pub fn init_sincos() {
    let sincos = unsafe { &mut SINCOS };

    let lenf = sincos.len() as f64;
    const TAU: f64 = std::f64::consts::TAU;
    for (i, x) in sincos.iter_mut().enumerate() {
        let f = (((i as f64) / lenf) * TAU).sin();

        *x = (f * QUANTIZER_LEVELS_F64) as i16;
    }
}

fn fp_undither(fp_x: u32) -> u32 {
    fp_x.wrapping_shr(PHASE_DITHER_POW as u32)
}

fn fixedp_sin(fp_x: u32) -> i16 {
    unsafe { *SINCOS.get_unchecked(fp_x as usize & FIXED_POINT_FRACTIONAL_MASK) }
}
fn fixedp_cos(fp_x: u32) -> i16 {
    unsafe {
        *SINCOS.get_unchecked((fp_x as usize + FIXED_POINT_90DEG) & FIXED_POINT_FRACTIONAL_MASK)
    }
}
pub fn phase_shift_per_sample_to_fixed_point(phase_shift: f32) -> u32 {
    (phase_shift as f64 * ((PHASE_DITHER * FIXED_POINT_FRACTIONAL) as f64)) as _
}
pub fn phase_shift_per_sample_to_fixed_point64(phase_shift: f64) -> u32 {
    (phase_shift * ((PHASE_DITHER * FIXED_POINT_FRACTIONAL) as f64)) as u32
}

pub fn fixed_point_to_phase_shift_per_sample(pulsation: u32) -> f32 {
    (pulsation as f64 / ((PHASE_DITHER * FIXED_POINT_FRACTIONAL) as f64)) as _
}

pub fn fixed_point_to_phase_shift_per_sample64(pulsation: u32) -> f64 {
    pulsation as f64 / ((PHASE_DITHER * FIXED_POINT_FRACTIONAL) as f64)
}

pub struct ChannelRing {
    pub ring_mask: usize,
    pub ring_offset: usize,
    pub ring_samples: Vec<i32>,
    pub ring_length: usize,
}
impl ChannelRing {
    pub fn new(ring_length: usize) -> Self {
        //let ring_length = 1 << 16;
        let ring_mask = ring_length - 1;
        assert_eq!(0, ring_mask & ring_length);
        let mut samples = Vec::with_capacity(ring_length);
        for _i in 0..ring_length {
            samples.push(0);
        }
        Self {
            ring_mask: ring_mask,
            ring_offset: 0,
            ring_samples: samples,
            ring_length: ring_length,
        }
    }

    pub fn reset(&mut self) {
        for x in self.ring_samples.iter_mut() {
            *x = 0;
        }
    }

    pub fn push_samples(&mut self, samples: &[f32]) {
        let ring_mask = self.ring_mask;
        let v = &mut self.ring_samples;
        let mut offset = self.ring_offset;

        for sample in samples {
            offset = (offset + 1) & ring_mask;
            //assert!((*s).abs() < 1.0);

            let sample_quantized = QUANTIZER_LEVELS_F * (*sample);

            #[cfg(debug_assertions)]
            {
                v[offset] = sample_quantized as i32;
            }

            #[cfg(not(debug_assertions))]
            unsafe {
                *(v.get_unchecked_mut(offset)) = sample_quantized as i32;
            }
        }
        self.ring_offset = offset;
    }
}

pub struct ChannelSWDFT {
    pub init_config: SpectrumConfig,
    pub config: SpectrumConfig,

    pub rings: Vec<ChannelRing>,

    pub rolling_gain: f64,
    pub current_power: f64,

    pub collect_every: usize,
    pub collect_frequency: usize,
    pub samples_to_collect_remaining: usize,

    //pub collected_spectrums: VecDeque<Arc<Mutex<Collected>>>,
    pub collected_spectrums_sender: Arc<Mutex<Sender<Collected>>>,
    pub collected_spectrums_receiver: Arc<Mutex<Option<Receiver<Collected>>>>,

    pub collector: Collector,
    pub collected_counter: usize,
    pub paused: bool,
    pub should_colorize: bool,

    pub spectrum_bins: SpectrumBins,
    pub measure_bins: VecDeque<MeasureBin>,
}

#[derive(Copy, Clone, TryFromPrimitive, IntoPrimitive, PartialEq)]
#[repr(u8)]
pub enum WindowType {
    Rect,
    BlackmanNutall,
    ExpBlackman,
    LogNormal,
}

pub struct Collector {
    pub windowtype: WindowType,
    pub kernel: Vec<i64>,
    pub kernel_sum: f64,
}

pub struct Collected {
    pub cur_rolling_gain: f64,
    pub spectrum: Vec<SSample>,
    pub peaks: Option<Vec<SPeak>>,
    pub rendered: Option<Vec<PosColVertex>>,
    pub snapshot: StateSnapshot,
}
#[derive(Clone)]
pub struct SColor {
    pub rgba: u32,
    pub rgb_gamma: (u16, u16, u16),
}

impl SColor {
    pub fn new(rgba: u32) -> SColor {
        let r = ((rgba >> 0) & 0xff) as u16;
        let g = ((rgba >> 8) & 0xff) as u16;
        let b = ((rgba >> 16) & 0xff) as u16;
        Self {
            rgba,
            rgb_gamma: (r * r, g * g, b * b),
        }
    }
}
pub struct SSample {
    pub value: f64,
    pub color: SColor,
    octave: f32,
}

pub struct RawSSample {
    pub complex: ComplexI64,
    pub color: SColor,
    //octave: f32,
    pub inv_lengthf: f64,
    pub a_weight: i64,
    pub length: u32,
}

pub struct SPeak {
    pub probe_index: i32,
    pub value: f64,
    pub color: SColor,
    pub octave: f32,
    pub alpha: f32,
}

#[derive(Clone)]
pub struct SpectrumConfig {
    pub sample_rate: u32,
    pub num_bins: u32,                  // 800
    pub max_f: f32,                     // 6.0
    pub min_f: f32,                     // 11000.0
    pub wave_cycles_resolution: f32,    // 45.0
    pub resolution_low_f_shelf_hz: f32, // 80.0
    pub subtraction_peaks: bool,
}

#[test]
fn test_a_weighting() {
    let w = a_weighting(40.0);

    assert_eq!(w, 100.0);
}

pub fn a_weighting(f: f32) -> f64 {
    let f = f as f64;
    let f2 = f * f;
    let a = (12194.0 as f64).powf(2.0) * f2 * f2;
    let s = (f2 + (107.7 as f64).powf(2.0)) * (f2 + (737.9 as f64).powf(2.0));
    let b = (f2 + (20.6 as f64).powf(2.0)) * s.sqrt() * (f2 + (12194.0 as f64).powf(2.0));

    //let zero_db_fix = 0.08; // good
    let zero_db_fix = 0.16;
    (a / b) + zero_db_fix
}

impl ChannelSWDFT {
    pub fn exp_interpolate(min: f32, max: f32, param: f32) -> f32 {
        let x = (min / max).ln();

        (x * (1.0 - param)).exp() * max
    }
    pub fn exp_inverse(min: f32, max: f32, param: f32) -> f32 {
        let min = min.ln();
        let max = max.ln();

        let param = param.ln();

        (param - min) / (max - min)
    }

    // x = (i as f32) / num_probesf
    pub fn num_probe_x_to_freq(c: &SpectrumConfig, x: f32) -> f32 {
        Self::exp_interpolate(c.min_f, c.max_f, x)
    }

    pub fn make_spectrum(num_bins: usize) -> VecDeque<RegularBin> {
        let mut spectrum_bins = VecDeque::new();

        for _i in 0..num_bins {
            spectrum_bins.push_back(RegularBin::new());
        }
        spectrum_bins
    }

    pub fn make_nc_spectrum(num_bins: usize) -> VecDeque<NCBin> {
        let mut spectrum_bins = VecDeque::new();

        for _i in 0..num_bins {
            spectrum_bins.push_back(NCBin::new());
        }
        spectrum_bins
    }

    pub fn init_regular_spectrum(c: &SpectrumConfig, bins: &mut VecDeque<RegularBin>) {
        let n = c.num_bins as f32;

        for i in 0..c.num_bins as usize {
            let freq = Self::num_probe_x_to_freq(c, i as f32 / n);
            Self::init_regular_bin(c, bins.get_mut(i).unwrap(), freq as f64);
        }
    }

    pub fn init_nc_spectrum(c: &SpectrumConfig, bins: &mut VecDeque<NCBin>) {
        let n = c.num_bins as f32;

        for i in 0..c.num_bins as usize {
            let freq = Self::num_probe_x_to_freq(c, i as f32 / n);
            Self::init_nc_bin(c, bins.get_mut(i).unwrap(), freq as f64);
        }
    }

    pub fn init_regular_bin(c: &SpectrumConfig, bin: &mut RegularBin, freq_hz: f64) {
        bin.meta.reinit(freq_hz, c.sample_rate);

        bin.bin.reinit_by_config(c, freq_hz, false);
    }

    pub fn init_nc_bin(c: &SpectrumConfig, bin: &mut NCBin, freq_hz: f64) {
        bin.meta.reinit(freq_hz, c.sample_rate);

        let sample_rate = c.sample_rate as f64;

        let window_size = DftBin::window_size(
            c.wave_cycles_resolution,
            sample_rate as f32,
            freq_hz as f32,
            c.resolution_low_f_shelf_hz,
        );

        let delta_freq = sample_rate / (window_size as f64);

        let fa = freq_hz as f64 - delta_freq * 0.5;
        let fb = freq_hz as f64 + delta_freq * 0.5;

        let pa = phase_shift_per_sample_to_fixed_point64(fa / sample_rate);
        let pb = phase_shift_per_sample_to_fixed_point64(fb / sample_rate);
        bin.bina.reinit_exact(fa, pa, window_size, false);
        bin.binb.reinit_exact(fb, pb, window_size, false);
    }

    pub fn reinit_my_spectrum(&mut self, config: &SpectrumConfig) {
        self.config = config.clone();

        Self::reinit_spectrum(&mut self.spectrum_bins, config);
    }

    pub fn reinit_spectrum(spectrum_bins: &mut SpectrumBins, config: &SpectrumConfig) {
        match spectrum_bins {
            SpectrumBins::DFT(dft_bins) => {
                Self::init_regular_spectrum(&config, dft_bins);
            }
            SpectrumBins::NC(nc_bins) => {
                Self::init_nc_spectrum(&config, nc_bins);
            }
        }
    }

    pub fn new(config: &SpectrumConfig) -> Self {
        init_sincos();
        //let sample_rate: usize = 24000;

        /*
        let end_freq = 200.0;
        let start_freq = 20.0;

        let num_probes = 500;
        for i in 0..num_probes {
            //let j = (i*i)/num_probes;
            let freq = start_freq + (end_freq - start_freq) * (i as f32) / (num_probes as f32);
            let sr_offset = (sample_rate as f32) / freq;
            probes.push(CorrProbe::new(sr_offset as usize, (sr_offset as usize) * 3));
        }*/

        //let iter_scale = max_f / num_probes as f32;

        //let mut dft_bins = Self::make_spectrum(config.num_bins as usize);
        //Self::init_dft_spectrum(&config, &mut dft_bins);

        //let nc_bins = Self::make_nc_spectrum(config.num_bins as usize);
        //Self::init_nc_spectrum(&config, &mut nc_bins);

        let mut rings = Vec::new();

        for i in 0..14 {
            let ring_size = 1 << 16;
            let ring_size = ring_size >> i;

            rings.push(ChannelRing::new(ring_size));
        }

        let measure_bins = VecDeque::new();
        let (sender, receiver) = std::sync::mpsc::channel();

        let mut s = Self {
            init_config: config.clone(),
            config: config.clone(),
            rings: rings,

            rolling_gain: 0.0001,
            current_power: 1.0,

            //spectrum_bins: SpectrumBins::DFT(spectrum_bins),
            spectrum_bins: Self::make_spectrum_bins(1, config),
            measure_bins,
            collect_every: 0,
            collect_frequency: 0,
            samples_to_collect_remaining: 1000,
            //collected_spectrums: VecDeque::new(),
            collected_spectrums_sender: Arc::new(Mutex::new(sender)),
            collected_spectrums_receiver: Arc::new(Mutex::new(Some(receiver))),
            collector: ChannelSWDFT::init_collector(6, WindowType::BlackmanNutall),
            collected_counter: 0,
            paused: false,
            should_colorize: false,
        };
        s.set_collect_frequency(60 * 10);
        //s.reinit_my_spectrum(&config);
        s
    }

    pub fn make_spectrum_bins(kind: u8, config: &SpectrumConfig) -> SpectrumBins {
        let mut bins = if kind == 0 {
            let bins = Self::make_spectrum(config.num_bins as usize);
            SpectrumBins::DFT(bins)
        } else {
            let bins = Self::make_nc_spectrum(config.num_bins as usize);
            SpectrumBins::NC(bins)
        };
        Self::reinit_spectrum(&mut bins, config);
        bins
    }

    pub fn set_collect_frequency(&mut self, collect_frequency: usize) {
        let collect_every = self.config.sample_rate as usize / collect_frequency;

        self.collect_every = ((collect_every >> 1) << 1).max(1);
        self.collect_frequency = collect_frequency;
        self.samples_to_collect_remaining = self.collect_every;
    }

    pub fn reset(&mut self) {
        match &mut self.spectrum_bins {
            SpectrumBins::DFT(dft_bins) => {
                for bin in dft_bins {
                    bin.bin.reset();
                }
            }
            _ => {}
        }
        for ring in &mut self.rings {
            ring.reset();
        }
    }

    pub fn pop_backfront<T>(q: &mut VecDeque<T>, back: bool) -> Option<T> {
        if back {
            q.pop_back()
        } else {
            q.pop_front()
        }
    }
    pub fn push_backfront<T>(q: &mut VecDeque<T>, v: T, back: bool) {
        if back {
            q.push_back(v)
        } else {
            q.push_front(v)
        }
    }

    pub fn recycle_bin(&mut self, left: bool, freq: f64) {
        match &mut self.spectrum_bins {
            SpectrumBins::DFT(dft_bins) => {
                let bin = Self::pop_backfront(dft_bins, left);
                let mut bin = bin.unwrap();

                Self::init_regular_bin(&self.config, &mut bin, freq);

                Self::push_backfront(dft_bins, bin, !left);
            }
            SpectrumBins::NC(nc_bins) => {
                let bin = Self::pop_backfront(nc_bins, left);
                let mut bin = bin.unwrap();

                Self::init_nc_bin(&self.config, &mut bin, freq);

                Self::push_backfront(nc_bins, bin, !left);
            }
        }
    }

    pub fn move_bins(&mut self, left: bool, amount: i32) {
        let num = amount;

        //println!("left: {}", left);
        if left {
            //let c = &self.config;
            let num_binsf = self.config.num_bins as f32;
            let mut moved = 0;
            for i in 0..num {
                let ii = i as f32;

                let freq = Self::num_probe_x_to_freq(&self.config, (-ii - 1.0) / num_binsf);
                if freq < self.init_config.min_f {
                    break;
                } else {
                    moved += 1;
                }

                self.recycle_bin(left, freq as f64);
            }
            let c = &mut self.config;
            let min_f = Self::num_probe_x_to_freq(c, (-moved as f32) / num_binsf);
            let max_f = Self::num_probe_x_to_freq(c, (-moved as f32 + num_binsf) / num_binsf);
            c.min_f = min_f;
            c.max_f = max_f;
        } else {
            let num_binsf = self.config.num_bins as f32;
            let mut moved = 0;
            for i in 0..num {
                let ii = i as f32;

                let freq = Self::num_probe_x_to_freq(&self.config, (ii + num_binsf) / num_binsf);
                if freq >= self.config.sample_rate as f32 / 2.0 {
                    break;
                } else {
                    moved += 1;
                }

                self.recycle_bin(left, freq as f64);
            }
            let c = &mut self.config;
            let min_f = Self::num_probe_x_to_freq(c, (moved as f32) / num_binsf);
            let max_f = Self::num_probe_x_to_freq(c, (moved as f32 + num_binsf) / num_binsf);
            c.min_f = min_f;
            c.max_f = max_f;
        }
    }

    pub fn blackman_nutall(x: f64) -> f64 {
        let tau = TAU as f64;
        // Blackman–Nuttall window
        let a0 = 0.3635819;
        let a1 = -0.4891775;
        let a2 = 0.1365995;
        let a3 = -0.0106411;

        let s0 = a0;
        let s1 = a1 * (1.0 * tau * x).cos();
        let s2 = a2 * (2.0 * tau * x).cos();
        let s3 = a3 * (3.0 * tau * x).cos();

        s0 + s1 + s2 + s3
    }

    #[allow(non_snake_case)]
    pub fn makewin_blackman_nutall(NN: i32) -> Vec<f64> {
        let mut window = Vec::new();

        for nn in 0..NN {
            let x = (nn as f64 + 0.5) / NN as f64;
            window.push(Self::blackman_nutall(x));
        }
        window
    }

    #[allow(non_snake_case)]
    pub fn makewin_exp(NN: i32) -> Vec<f64> {
        let mut window = Vec::new();

        for nn in 0..NN {
            let x = (nn as f64 + 0.5) / NN as f64;
            window.push((-x * 2.0).exp());
            //gkernel.push(1.0-x);
            //gkernel.push(0.9);
        }
        window
    }

    #[allow(non_snake_case)]
    pub fn makewin_expmod(NN: i32) -> Vec<f64> {
        let mut window = Vec::new();

        let blackmanize = 0.5;
        for nn in 0..NN {
            let x = (nn as f64 + 0.5) / NN as f64;
            window.push(
                (-x * 2.3).exp() * ((1.0 - blackmanize) + blackmanize * Self::blackman_nutall(x)),
            );
        }
        window
    }

    // https://www.google.com/search?q=y%3D(1%2Fx)*exp(-(ln(x)^2)%2F2)
    #[allow(non_snake_case)]
    pub fn makewin_lognormal(NN: i32) -> Vec<f64> {
        let mut window = Vec::new();

        let cutoffx = 5.0;
        let sigma = 0.8;

        let mut max_val: f64 = 0.000001;
        for nn in 0..NN {
            //y=(1/x)*exp(-(ln(x)^2)/2)

            let x = (nn as f64 + 0.5) / NN as f64;
            let x = x * cutoffx;

            let v = (1.0 / (x * sigma)) * (-x.ln().powf(2.0) / (2.0 * sigma * sigma)).exp();

            max_val = max_val.max(v);
            window.push(v);
        }
        for x in window.iter_mut() {
            *x /= max_val;
        }

        window
    }

    pub fn init_collector(n: i32, windowtype: WindowType) -> Collector {
        //let gkernel = vec![0.99];

        //let gkernel = vec![0.067234, 0.124009, 0.179044, 0.20236, 0.179044, 0.124009, 0.067234, 0.028532];

        let gkernel = match windowtype {
            WindowType::Rect => vec![0.99],
            WindowType::BlackmanNutall => Self::makewin_blackman_nutall(n),
            WindowType::ExpBlackman => Self::makewin_expmod(n),
            WindowType::LogNormal => Self::makewin_lognormal(n),
        };

        let kernel: Vec<i64> = gkernel
            .iter()
            .map(|x| (x * (0xFFF0 as f64)) as i64)
            .collect();
        let mut kernel_sum = 0.0;
        for &f in kernel.iter() {
            kernel_sum += f as f64 / (0xFFF0 as f64);
        }
        kernel_sum /= kernel.len() as f64;
        // crate::klog!("kernel_sum: {}", kernel_sum);
        // crate::klog!("kernel: {:?}", kernel);
        // crate::klog!("kernel[0]: {:?}", kernel[0] as i64);

        Collector {
            windowtype,
            kernel,
            kernel_sum,
        }
    }

    pub fn smoothed(spectrum: &Vec<SSample>) -> Vec<f32> {
        (0..spectrum.len())
            .map(|i| {
                const RANGE: i32 = 2;
                const RANGE_INV: f32 = 1.0 / (RANGE * 2 + 1) as f32;
                let sum: f32 = (-RANGE..RANGE)
                    .map(|j| {
                        let k: i32 = i as i32 + j;
                        if k >= 0 {
                            spectrum.get(k as usize)
                        } else {
                            None
                        }
                    })
                    .filter_map(|x| x)
                    .map(|x| x.value as f32)
                    .sum();
                sum * RANGE_INV
            })
            .collect()
    }

    pub fn collect_spectrum(&self) -> Collected {
        //const GKERNEL: [f32; 9] = [0.028532, 0.067234, 0.124009, 0.179044, 0.20236, 0.179044, 0.124009, 0.067234, 0.028532];
        //const GKERNEL: [f32; 9] = [0.0, 0.0, 0.0, 0.0, 0.20236, 0.0, 0.0, 0.0, 0.0];

        //let mut spectrum = Vec::new();

        let kernel = &self.collector.kernel;
        let kernel_sum = self.collector.kernel_sum;

        let subtraction_peaks = self.config.subtraction_peaks;
        use rayon::prelude::*;
        let (mut spectrum, mut peaks) = rayon::join(
            || {
                let spectrum: Vec<SSample> = match &self.spectrum_bins {
                    SpectrumBins::DFT(dft_bins) => {
                        dft_bins
                            .par_iter()
                            .map(|s| {
                                let (mut magnitude, _) = s.bin.sum_magnitude_kerneled(kernel);

                                magnitude = magnitude.sqrt();
                                magnitude /= kernel_sum;
                                magnitude *= s.meta.a_weight;
                                //val /= s.length as f64;

                                SSample {
                                    value: magnitude,
                                    color: s.meta.color.clone(),
                                    octave: s.meta.octave,
                                }
                            })
                            .collect()
                    }
                    SpectrumBins::NC(nc_bins) => {
                        nc_bins
                            .par_iter()
                            .map(|s| {
                                // NC method
                                let a = s.bina.sum_ranged_all();
                                let b = s.binb.sum_ranged_all();

                                let a = a.to_f64();
                                let b = b.to_f64();

                                //const TAU: f64 = std::f64::consts::TAU;
                                let phasea = -(s.bina.current_phase as f64)
                                    * FIXED_POINT_PHASE_MULTIPLIER_TAU;
                                let phaseb = -(s.binb.current_phase as f64)
                                    * FIXED_POINT_PHASE_MULTIPLIER_TAU;

                                let a = a.rotate(phasea);
                                let b = b.rotate(phaseb);
                                let mut magnitude = -(a.re * b.re + a.im * b.im);
                                if magnitude < 0.0 {
                                    magnitude = 0.0;
                                }
                                magnitude = magnitude.sqrt();
                                magnitude *= s.meta.a_weight;
                                magnitude /= s.bina.length as f64;

                                // if octave.is_nan() {
                                //     println!(
                                //         "ocatave: {} is nan, at {} hz",
                                //         octave,
                                //         s.bina.frequency64(self.init_config.sample_rate as f64)
                                //     );
                                // }
                                SSample {
                                    value: magnitude,
                                    color: s.meta.color.clone(),
                                    octave: s.meta.octave,
                                }
                            })
                            .collect()
                    }
                };

                spectrum
            },
            || {
                if subtraction_peaks {
                    if let SpectrumBins::DFT(dft_bins) = &self.spectrum_bins {
                        let mut raw_spectrum: Vec<RawSSample> = dft_bins
                            .par_iter()
                            .map(|s| {
                                let complex = s.bin.sum_ranged_all();

                                RawSSample {
                                    complex,
                                    color: s.meta.color.clone(),
                                    //octave: s.octave,
                                    inv_lengthf: 1.0 / s.bin.lengthf,
                                    a_weight: (s.meta.a_weight * s.meta.a_weight * 10000.0) as i64,
                                    length: s.bin.length as u32,
                                }
                            })
                            .collect();

                        Some(self.subtraction_peaks(&mut raw_spectrum))
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
        );
        if true {
            match peaks {
                None => {
                    peaks = Some(self.simple_peaks(&spectrum));
                }
                _ => {}
            }
        }
        //let mut peaks = peaks.unwrap();
        match &mut peaks {
            Some(peaks) => {
                peaks.sort_by(|a, b| a.octave.partial_cmp(&b.octave).unwrap());
                if self.should_colorize {
                    Self::colorize_spectrum(&mut spectrum, peaks);
                } else {
                    //Self::decolorize_spectrum(&mut spectrum);
                }
            }
            _ => {}
        }

        let snapshot = StateSnapshot {
            current_algo: self.spectrum_bins.state(),
            window_type: self.collector.windowtype,
            collect_every: self.collect_every,
            collect_frequency: self.collect_frequency,
            window_kernel_len: self.collector.kernel.len(),
        };

        Collected {
            cur_rolling_gain: self.rolling_gain,
            spectrum,
            peaks,
            rendered: None,
            snapshot,
        }
    }

    pub fn subtraction_peaks(&self, spectrum: &mut Vec<RawSSample>) -> Vec<SPeak> {
        let mut peaks = Vec::new();

        let n = 15;
        for j in 0..n {
            let mut peak = self.subtract_peak(spectrum);
            peak.alpha = 1.0 - j as f32 / n as f32;
            peaks.push(peak);
        }
        peaks
    }

    pub fn subtract_peak(&self, spectrum: &mut Vec<RawSSample>) -> SPeak {
        //let sample_ratef = self.config.sample_rate as f32;

        if let SpectrumBins::DFT(dft_bins) = &self.spectrum_bins {
            let max_index = Self::find_max_ssample(spectrum);
            let s = &spectrum[max_index];
            let bin = &dft_bins[max_index];
            let octave = bin.meta.octave;
            //let val = s.value;

            //let mag = s.complex.magnitude_squared();
            let c = s.complex.to_f64();
            let mag = c.magnitude_squared();
            let mag = mag.sqrt();

            let angle = c.im.atan2(c.re);

            const TAU: f64 = std::f64::consts::TAU;

            let subtracted_phase =
                angle / TAU - (bin.bin.current_phase as f64 * FIXED_POINT_PHASE_MULTIPLIER);

            let subtracted_pulse = bin.bin.pulsation;
            //let subtracted_freq = self.spectrum_bins[max_index].frequency(sample_ratef);

            let peak = SPeak {
                value: mag * bin.meta.a_weight * s.inv_lengthf,
                probe_index: max_index as i32,
                color: spectrum[max_index].color.clone(),
                octave: octave,
                alpha: 1.0,
            };

            Self::subtract_sine(
                max_index as i32,
                subtracted_pulse,
                mag,
                subtracted_phase,
                spectrum,
                &dft_bins,
            );
            peak
        } else {
            unimplemented!();
        }
    }

    pub fn find_max_ssample(spectrum: &Vec<RawSSample>) -> usize {
        let mut max_val = 0;
        let mut max_index: usize = 0;
        spectrum.iter().enumerate().for_each(|(i, s)| {
            let mut c = s.complex.clone();
            c.re /= QUANTIZER_LEVELS;
            c.im /= QUANTIZER_LEVELS;

            c.re /= s.length as i64;
            c.im /= s.length as i64;

            let mag = c.magnitude_squared();
            //let mag = mag*s.a_weight;
            if mag > max_val {
                max_val = mag;
                max_index = i;
            }
        });

        max_index
    }

    // integrate sin(at+c)cos(ct+d)dt
    #[inline]
    pub fn integral_cos(t1: f64, t2: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
        let diff = a - c;
        let phases = b - d;
        let arg1 = (t1 * diff) + phases;
        let arg2 = (t2 * diff) + phases;

        -0.5 * ((arg1.cos() - arg2.cos()) / (diff + 1e-8))
    }
    // integrate sin(at+c)sin(ct+d)dt
    #[inline]
    pub fn integral_sin(t1: f64, t2: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
        let diff = a - c;
        let phases = b - d;
        let arg1 = (t1 * diff) + phases;
        let arg2 = (t2 * diff) + phases;

        -0.5 * ((arg1.sin() - arg2.sin()) / (diff + 0.000000001))
    }

    // full period is from 0 to 1
    pub fn fast_cos(x: f32) -> f32 {
        let x = x % 1.0;
        let x = x + 1.0;
        let x = x % 1.0;
        let x = x * (FIXED_POINT_FRACTIONAL as f32);
        let x = x as u32;
        (fixedp_cos(x) as f32) * QUANTIZER_LEVELS_INV_F
    }

    // full period is from 0 to 1
    pub fn fast_sin(x: f32) -> f32 {
        let x = x % 1.0;
        let x = x + 1.0;
        let x = x % 1.0;
        let x = x * (FIXED_POINT_FRACTIONAL as f32);
        let x = x as u32;
        (fixedp_sin(x) as f32) * QUANTIZER_LEVELS_INV_F
    }

    #[inline]
    pub fn integral_cos2(t2: f64, a: f64, b: i64, c: f64, d: i64) -> f64 {
        let diff = a - c;
        let phases = b - d;
        let arg1 = phases;
        let arg2 = ((t2 * diff) * (PHASE_DITHER * FIXED_POINT_FRACTIONAL) as f64) as i64 + phases;
        const TAU: f64 = std::f64::consts::TAU;

        let x = fixedp_cos((arg1 / PHASE_DITHER as i64) as u32) as i32
            - fixedp_cos((arg2 / PHASE_DITHER as i64) as u32) as i32;
        -0.5 * (x as f32 * QUANTIZER_LEVELS_INV_F) as f64 / (TAU * diff + 0.000000001)
    }

    #[inline]
    pub fn integral_sin2(t2: f64, a: f64, b: i64, c: f64, d: i64) -> f64 {
        let diff = a - c;
        let phases = b - d;
        let arg1 = phases;
        let arg2 = ((t2 * diff) * (PHASE_DITHER * FIXED_POINT_FRACTIONAL) as f64) as i64 + phases;
        const TAU: f64 = std::f64::consts::TAU;

        let x = fixedp_sin((arg1 / PHASE_DITHER as i64) as u32) as i32
            - fixedp_sin((arg2 / PHASE_DITHER as i64) as u32) as i32;
        -0.5 * (x as f32 * QUANTIZER_LEVELS_INV_F) as f64 / (TAU * diff + 0.000000001)
    }

    pub fn subtract_sine(
        max_index: i32,
        subtracted_pulse: u32,
        mag: f64,
        subtracted_phase: f64,
        spectrum: &mut Vec<RawSSample>,
        bins: &VecDeque<RegularBin>,
    ) {
        let subtracted_f = fixed_point_to_phase_shift_per_sample(subtracted_pulse) as f64;
        let subtracted_f = subtracted_f;

        let fixedp_phase_mul = 1.0 / (PHASE_DITHER as f64 * FIXED_POINT_FRACTIONAL as f64);

        const TAU: f64 = std::f64::consts::TAU;

        use rayon::prelude::*;

        let ai = max_index - 120;
        let bi = max_index + 120;

        let ai = ai.max(0) as usize;
        let bi = bi.min(bins.len() as i32) as usize;

        spectrum[ai..bi]
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, s)| {
                let i = ai + i;

                if i as i32 == max_index {
                    s.complex.re = 0;
                    s.complex.im = 0;
                } else {
                    let bin = &bins[i];
                    let ll = bin.bin.lengthf;

                    let f1 = fixed_point_to_phase_shift_per_sample(bin.bin.pulsation) as f64;
                    let f2 = subtracted_f;

                    let p1 = -(bin.bin.current_phase as f64 * fixedp_phase_mul);
                    let p2 = subtracted_phase;

                    let aac = Self::integral_cos(0.0, ll, TAU * f1, TAU * p1, TAU * f2, TAU * p2);
                    let aas = Self::integral_sin(0.0, ll, TAU * f1, TAU * p1, TAU * f2, TAU * p2);

                    let ss = bin.bin.inv_lengthf * aas * mag * 2.0;
                    let cc = bin.bin.inv_lengthf * aac * mag * 2.0;

                    s.complex.re -= (ss * QUANTIZER_LEVELS_F64) as i64;
                    s.complex.im -= (cc * QUANTIZER_LEVELS_F64) as i64;
                }
            });
    }

    pub fn subtract_sine2(
        max_index: i32,
        subtracted_pulse: u32,
        mag: f64,
        subtracted_phase: f64,
        spectrum: &mut Vec<RawSSample>,
        bins: &VecDeque<DftBin>,
    ) {
        let subtracted_phase =
            phase_shift_per_sample_to_fixed_point(subtracted_phase as f32) as i64;

        let subtracted_f = fixed_point_to_phase_shift_per_sample(subtracted_pulse) as f64;
        let subtracted_f = subtracted_f + 0.01 / 24000.0;

        use rayon::prelude::*;

        let ai = max_index - 120;
        let bi = max_index + 120;

        let ai = ai.max(0) as usize;
        let bi = bi.min(bins.len() as i32) as usize;

        spectrum[ai..bi]
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, s)| {
                let i = ai + i;

                if i as i32 == max_index {
                    s.complex.re = 0;
                    s.complex.im = 0;
                } else {
                    let bin = &bins[i];
                    let ll = bin.lengthf;

                    let f1 = fixed_point_to_phase_shift_per_sample(bin.pulsation) as f64;
                    let f2 = subtracted_f;

                    let p1 = (-(bin.current_phase as i64))
                        & ((PHASE_DITHER * FIXED_POINT_FRACTIONAL) as i64 - 1); //-(bin.current_phase as f64 * fixedp_phase_mul);
                    let p2 = subtracted_phase;

                    let aac = Self::integral_cos2(ll, f1, p1, f2, p2);
                    let aas = Self::integral_sin2(ll, f1, p1, f2, p2);

                    let ss = bin.inv_lengthf * aas * mag * 2.0;
                    let cc = bin.inv_lengthf * aac * mag * 2.0;

                    s.complex.re -= (ss * QUANTIZER_LEVELS_F64) as i64;
                    s.complex.im -= (cc * QUANTIZER_LEVELS_F64) as i64;
                }
            });
    }

    pub fn simple_peaks(&self, spectrum: &Vec<SSample>) -> Vec<SPeak> {
        let smooth_spectrum = Self::smoothed(&spectrum);

        let mut last_val = -1.0;
        let mut lastlast_val = -2.0;

        let mut peaks = Vec::new();
        smooth_spectrum.iter().enumerate().for_each(|(i, s)| {
            let val = *s;

            //if val > 0.000001 && val < last_val && last_val > lastlast_val {
            if val < last_val && last_val > lastlast_val {
                let octave = spectrum[i].octave; //self.spectrum_bins[i].octave;

                let real_val = spectrum[i].value;
                peaks.push(SPeak {
                    value: real_val,
                    probe_index: i as i32,
                    color: spectrum[i].color.clone(),
                    octave: octave,
                    alpha: 1.0,
                });

                lazy_static! {
                    static ref LOG2_3: f32 = (3.0 as f32).log2();
                    static ref LOG2_5: f32 = (5.0 as f32).log2();
                    static ref LOG2_6: f32 = (6.0 as f32).log2();
                    static ref LOG2_7: f32 = (7.0 as f32).log2();
                }

                //3x
                // peaks.push(SPeak {
                //     value: val*(1.0/(3.0)),
                //     probe_index: i as i32,
                //     color: spectrum[i].color,
                //     octave: octave + *LOG2_3,
                // });

                // // 5x
                // peaks.push(SPeak {
                //     value: val*(1.0/(5.0)),
                //     probe_index: i as i32,
                //     color: spectrum[i].color,
                //     octave: octave + *LOG2_5,
                // });

                // // 6x
                // peaks.push(SPeak {
                //     value: val*(1.0/(6.0)),
                //     probe_index: i as i32,
                //     color: spectrum[i].color,
                //     octave: octave + *LOG2_6,
                // });

                // // 7x
                // peaks.push(SPeak {
                //     value: val*(1.0/(7.0)),
                //     probe_index: i as i32,
                //     color: spectrum[i].color,
                //     octave: octave + *LOG2_7,
                // });
            }
            lastlast_val = last_val;
            last_val = val;
        });
        peaks
    }

    pub fn colorize_spectrum_old(spectrum: &mut Vec<SSample>, peaks: &Vec<SPeak>) {
        let mut pindex = 0;
        let mut current = peaks.get(pindex);
        let mut next = peaks.get(pindex + 1);
        for (i, s) in spectrum.iter_mut().enumerate() {
            match &current {
                None => {}
                Some(peak) => {
                    s.color = peak.color.clone();

                    match &next {
                        None => {}
                        Some(npeak) => {
                            let to_last = i as i32 - peak.probe_index;
                            let to_next = npeak.probe_index - i as i32;

                            let to_next = to_next as f32;
                            let to_last = to_last as f32;
                            let ratio = (peak.value / npeak.value) as f32;

                            if to_next * ratio <= to_last {
                                current = next;
                                pindex += 1;
                                next = peaks.get(pindex + 1);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn colorize_spectrum(spectrum: &mut Vec<SSample>, _peaks: &Vec<SPeak>) {
        // let mut pindex = 0;
        // let mut current = peaks.get(pindex);
        // let mut next = peaks.get(pindex + 1);
        let range = 64;
        let half_range = range / 2;
        let len = spectrum.len();
        // (0..len).into_iter().map(|i|{
        //     let a = i - range;
        //     let b = i + range;
        //     let nearby = &spectrum[a.max(0)..b.min(len)];

        //     let value = nearby.map(|s| s.value)
        //     //s.color = s.color;
        //     (i, value)
        // }).for_each(|(i, value)|{

        // });

        struct RunningAvg {
            col: (u64, u64, u64),
            vsuma: u64,
        }

        let mut avg = RunningAvg {
            col: (0, 0, 0),
            vsuma: 1,
        };

        let add_sample = |s: &SSample, avg: &mut RunningAvg| {
            let uval = (s.value * s.value * 10000.0) as u64;
            avg.vsuma += uval;
            avg.col.0 += uval * s.color.rgb_gamma.0 as u64;
            avg.col.1 += uval * s.color.rgb_gamma.1 as u64;
            avg.col.2 += uval * s.color.rgb_gamma.2 as u64;
        };

        let del_sample = |s: &SSample, avg: &mut RunningAvg| {
            let uval = (s.value * s.value * 10000.0) as u64;
            avg.vsuma -= uval;
            avg.col.0 -= uval * s.color.rgb_gamma.0 as u64;
            avg.col.1 -= uval * s.color.rgb_gamma.1 as u64;
            avg.col.2 -= uval * s.color.rgb_gamma.2 as u64;
        };

        for i in 0..half_range {
            add_sample(&spectrum[i], &mut avg);
        }
        for i in 0..len {
            if true {
                use integer_sqrt::IntegerSquareRoot;
                let rr = (avg.col.0 / avg.vsuma) as u32;
                let gg = (avg.col.1 / avg.vsuma) as u32;
                let bb = (avg.col.2 / avg.vsuma) as u32;

                let rr = (rr.integer_sqrt().min(0xff)) as u32;
                let gg = (gg.integer_sqrt().min(0xff)) as u32;
                let bb = (bb.integer_sqrt().min(0xff)) as u32;

                let calc_rgba = 0xff000000 | (rr) | (gg << 8) | (bb << 16);

                spectrum[i].color.rgba = calc_rgba;

                let a = i - half_range;
                let b = i + half_range;

                spectrum.get(a).map(|s| del_sample(s, &mut avg));
                spectrum.get(b).map(|s| add_sample(s, &mut avg));
            } else {
                use integer_sqrt::IntegerSquareRoot;
                let rr = (spectrum[i].color.rgb_gamma.0.integer_sqrt().min(0xff)) as u32;
                let gg = (spectrum[i].color.rgb_gamma.1.integer_sqrt().min(0xff)) as u32;
                let bb = (spectrum[i].color.rgb_gamma.2.integer_sqrt().min(0xff)) as u32;

                let calc_rgba = 0xff000000 | (rr) | (gg << 8) | (bb << 16);

                spectrum[i].color = SColor::new(calc_rgba);
            }
        }
    }

    pub fn decolorize_spectrum(spectrum: &mut Vec<SSample>) {
        for (_i, s) in spectrum.iter_mut().enumerate() {
            s.color = SColor::new(0xFFFFFFFF);
        }
    }

    pub fn colorize_spectrum_simple(spectrum: &mut Vec<SSample>) {
        for (_i, s) in spectrum.iter_mut().enumerate() {
            s.color = SColor::new(0xFFFFFFFF);
        }
    }

    pub fn colorize_spectrum_near(spectrum: &mut Vec<SSample>, peaks: &Vec<SPeak>) {
        let max_diff = 0.31;

        let mut peak_index = 0;
        for (_i, s) in spectrum.iter_mut().enumerate() {
            while peak_index < peaks.len() && peaks[peak_index].octave + max_diff < s.octave {
                peak_index += 1;
            }

            let mut end_index = peak_index + 100;
            if end_index >= peaks.len() {
                end_index = peaks.len();
            }
            let color = Self::weighted_color(s.octave, max_diff, &peaks[peak_index..end_index]);
            s.color = SColor::new(color);
            //s.color = 0xFF000000 | color;

            //s.color = 0xFFFFFFFF;
        }
    }

    pub fn weighted_color(center_octave: f32, max_diff: f32, peaks: &[SPeak]) -> u32 {
        let initc = 0.00001;
        let mut r = initc;
        let mut g = initc;
        let mut b = initc;
        let mut weight_sum = initc;

        for peak in peaks {
            let rgba = peak.color.rgba;
            let pr = rgba & 0xFF;
            let pg = (rgba >> 8) & 0xFF;
            let pb = (rgba >> 16) & 0xFF;

            let mut weight = peak.octave - center_octave;
            if weight < 0.0 {
                if weight < -max_diff {
                    break;
                }
                weight = -weight;
            }
            if weight < max_diff {
                let weight = 1.0 - (weight / max_diff) as f64;
                let weight = weight * weight;
                let weight = weight * weight;
                let val = peak.value;
                let weight = weight * val * val;
                weight_sum += weight;

                let pr = pr as f64 / 255.0;
                let pg = pg as f64 / 255.0;
                let pb = pb as f64 / 255.0;

                let pr = pr * pr;
                let pg = pg * pg;
                let pb = pb * pb;

                let pr = pr as f64 * weight;
                let pg = pg as f64 * weight;
                let pb = pb as f64 * weight;

                r += pr;
                g += pg;
                b += pb;
            }
        }
        r /= weight_sum;
        g /= weight_sum;
        b /= weight_sum;

        r = r.sqrt();
        g = g.sqrt();
        b = b.sqrt();

        r = r.min(1.0);
        g = g.min(1.0);
        b = b.min(1.0);

        r *= 255.0;
        g *= 255.0;
        b *= 255.0;

        let r = r as u32;
        let g = g as u32;
        let b = b as u32;

        let color = r | (g << 8) | (b << 16);

        color
    }

    pub fn half_samplerate(input: &[f32]) -> Vec<f32> {
        let half_len = input.len() / 2;
        let mut input_2 = Vec::with_capacity(half_len);

        for i in 0..input.len() / 2 {
            let a = input[i * 2];
            let b = input[i * 2 + 1];
            input_2.push((a + b) * 0.5);
        }
        input_2
    }
    pub fn multihalved_signal<F>(octaves: usize, main_input: &[f32], mut cb: F)
    where
        F: FnMut(usize, &[f32]),
    {
        let mut vecs: Vec<Vec<f32>> = Vec::new();
        for i in 0..octaves {
            let signal = if i == 0 { main_input } else { &vecs[i - 1] };
            let half = Self::half_samplerate(signal);

            {
                let octave = i;
                cb(octave, &half);
            }
            vecs.push(half);
        }
    }

    pub fn process_input(&mut self, main_input: &[f32]) {
        let rings = &mut self.rings;
        // Self::multihalved_signal(rings.len(), main_input, |octave, samples| {
        //     rings[octave].push_samples(samples);
        // });

        let samples = Self::half_samplerate(main_input);

        rings[1].push_samples(&samples);

        let r = &rings[1];
        let l = main_input.len() / 2;

        {
            use rayon::prelude::*;
            match &mut self.spectrum_bins {
                SpectrumBins::DFT(dft_bins) => {
                    dft_bins.par_iter_mut().for_each(|bin| {
                        bin.bin.advance(l, &r.ring_samples, r.ring_offset);
                    });
                }
                SpectrumBins::NC(nc_bins) => {
                    nc_bins.par_iter_mut().for_each(|bin| {
                        //let ring = rings[bin.meta.samplerate_octave];
                        bin.bina.advance(l, &r.ring_samples, r.ring_offset);
                        bin.binb.advance(l, &r.ring_samples, r.ring_offset);
                    });
                }
            }
        }
        {
            self.measure_bins.iter_mut().for_each(|probe| {
                probe.bin.advance(l, &r.ring_samples, r.ring_offset);
                probe.sample_counter += main_input.len() as i64;
            });
        }
    }

    pub fn power_of_signal(samples: &[f32]) -> f64 {
        let mut power_sum: f64 = 0.0;
        for x in samples {
            let xx = *x as f64;
            power_sum += xx * xx;
        }

        power_sum.sqrt() / (samples.len() as f64)
    }

    pub fn power_of_spectrum(samples: &[SSample]) -> f64 {
        let mut power_sum: f64 = 0.0;
        for x in samples {
            let xx = x.value;
            power_sum += xx * xx;
        }

        power_sum.sqrt() / (samples.len() as f64)
    }

    pub fn on_input(&mut self, input: &[f32]) {
        //let len = self.last_samples.len();

        const SPECTRUM_POWER_AGC: bool = true;
        if !SPECTRUM_POWER_AGC {
            self.current_power = Self::power_of_signal(input);
            self.rolling_gain += (self.current_power + 0.00001 - self.rolling_gain) * 0.01;
        }

        if !self.paused {
            let mut remaining = input;
            while remaining.len() > 0 {
                let mut rem = self.samples_to_collect_remaining;
                if remaining.len() < rem {
                    rem = remaining.len();
                }
                let (now_process, next) = remaining.split_at(rem);
                self.process_input(now_process);
                remaining = next;
                self.samples_to_collect_remaining -= rem;

                if self.samples_to_collect_remaining <= 0 {
                    self.samples_to_collect_remaining = self.collect_every;

                    let spectrum = self.collect_spectrum();
                    if SPECTRUM_POWER_AGC
                        && self.collected_counter % (self.collect_frequency / 60) == 0
                    {
                        self.current_power = Self::power_of_spectrum(&spectrum.spectrum);
                        self.rolling_gain +=
                            (self.current_power + 0.00001 - self.rolling_gain) * 0.01;
                    }

                    // self.collected_spectrums
                    //     .push_front(Arc::new(Mutex::new(spectrum)));
                    let _res = self
                        .collected_spectrums_sender
                        .lock()
                        .unwrap()
                        .send(spectrum);
                    self.collected_counter += 1;
                    // if self.collected_spectrums.len() > 200 {
                    //     self.collected_spectrums.pop_back();
                    // }
                }
            }
        }
        //console_log!("self.last_samples.len(): {}", self.last_samples.len())
    }
}

mod framework;

#[cfg(not(target_arch = "wasm32"))]
pub mod adevice_cpal;
// #[cfg(not(target_arch = "wasm32"))]
// use device_cpal::SlidingCpal;
// #[cfg(not(target_arch = "wasm32"))]
// use device_cpal::SlidingImplReceiver;

#[cfg(target_arch = "wasm32")]
pub mod adevice_web;

use crate::spectrumapp::spectrumui::SlidingMainThread;
use kikod::Kikod;
use std::collections::VecDeque;
use std::sync::Mutex;
use winit::event::VirtualKeyCode;

use spectrumui::SpectrumUI;

pub mod displayparams;
pub mod fontrenderer;
pub mod kikod;
pub mod myvertex;
pub mod sbswdft;
pub mod spectrumui;
pub mod texture;

#[cfg(target_arch = "wasm32")]
pub mod pool;

pub mod kwasm;
#[cfg(target_arch = "wasm32")]
pub mod tracingalloc;
#[cfg(target_arch = "wasm32")]
pub mod wasm_rayon;

// #[cfg(feature = "rawwebgl")]
// pub mod rawwebgl;

use fontrenderer::FontAtlas;
use fontrenderer::FontRenderer;

use myvertex::*;
use sbswdft::ChannelSWDFT;
use sbswdft::SlidingImpl;
use sbswdft::SpectrumConfig;
use texture::KRGBAImage;
use texture::KRect;

use core::num::NonZeroU32;
//use parking_lot::Mutex;
use std::borrow::Cow;
use std::cell::RefCell;

use std::rc::Rc;
use std::sync::Arc;
use std::sync::Weak;
use wgpu::util::DeviceExt;

use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn console_log_str(s: &str);

}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! klog {
    ($($t:tt)*) => (println!($($t)*))
}

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! klog {
    ($($t:tt)*) => ($crate::spectrumapp::console_log_str(&format!($($t)*).as_str()))
}

#[macro_export]
macro_rules! kwarn {
    ($($t:tt)*) => (println!($($t)*))
}

#[macro_export]
macro_rules! include_str_or_read_at_runtime {
    ($file:expr $(,)?) => {{
        {
            let static_str = include_str!($file);

            let mut path = std::path::PathBuf::from("src/spectrumapp/");
            path.push($file);
            let contents: String = std::fs::read_to_string(path).unwrap_or(static_str.to_string());
            //.expect("read_to_string: include_str_or_read_at_runtime");

            contents
        }
    }};
}

pub static mut PANICKED: bool = false;

pub fn panicked() -> bool {
    unsafe { PANICKED }
}

pub trait PCMReceiver: Send + Sync {
    fn num_channels(&self) -> usize;
    fn on_receive(&self, samples: &[Vec<f32>]);
}

pub struct SlidingImplReceiver {
    weak_sliding_channels: Vec<Weak<Mutex<SlidingImpl>>>,
}
impl SlidingImplReceiver {
    pub fn new(channels: Vec<Weak<Mutex<SlidingImpl>>>) -> Self {
        Self {
            weak_sliding_channels: channels,
        }
    }
}

impl PCMReceiver for SlidingImplReceiver {
    fn num_channels(&self) -> usize {
        self.weak_sliding_channels.len()
    }

    fn on_receive(&self, channels_samples: &[Vec<f32>]) {
        for (out_channel, samples) in self.weak_sliding_channels.iter().zip(channels_samples) {
            out_channel.upgrade().map(|strong| {
                match &mut *strong.lock().unwrap() {
                    SlidingImpl::DFT(dft) => dft.on_input(&samples),
                    //SlidingImpl::Correlator(corr) => corr.on_input(&buf),
                    //_ => {}
                };
            });
        }
    }
}

#[allow(unused)]
fn keycode2kikod(vkeycode: VirtualKeyCode) -> Kikod {
    match vkeycode {
        VirtualKeyCode::Key1 => Kikod::Key1,
        VirtualKeyCode::Key2 => Kikod::Key2,
        VirtualKeyCode::Key3 => Kikod::Key3,
        VirtualKeyCode::Key4 => Kikod::Key4,
        VirtualKeyCode::Key5 => Kikod::Key5,
        VirtualKeyCode::Key6 => Kikod::Key6,
        VirtualKeyCode::Key7 => Kikod::Key7,
        VirtualKeyCode::Key8 => Kikod::Key8,
        VirtualKeyCode::Key9 => Kikod::Key9,
        VirtualKeyCode::Key0 => Kikod::Key0,
        VirtualKeyCode::A => Kikod::A,
        VirtualKeyCode::B => Kikod::B,
        VirtualKeyCode::C => Kikod::C,
        VirtualKeyCode::D => Kikod::D,
        VirtualKeyCode::E => Kikod::E,
        VirtualKeyCode::F => Kikod::F,
        VirtualKeyCode::G => Kikod::G,
        VirtualKeyCode::H => Kikod::H,
        VirtualKeyCode::I => Kikod::I,
        VirtualKeyCode::J => Kikod::J,
        VirtualKeyCode::K => Kikod::K,
        VirtualKeyCode::L => Kikod::L,
        VirtualKeyCode::M => Kikod::M,
        VirtualKeyCode::N => Kikod::N,
        VirtualKeyCode::O => Kikod::O,
        VirtualKeyCode::P => Kikod::P,
        VirtualKeyCode::Q => Kikod::Q,
        VirtualKeyCode::R => Kikod::R,
        VirtualKeyCode::S => Kikod::S,
        VirtualKeyCode::T => Kikod::T,
        VirtualKeyCode::U => Kikod::U,
        VirtualKeyCode::V => Kikod::V,
        VirtualKeyCode::W => Kikod::W,
        VirtualKeyCode::X => Kikod::X,
        VirtualKeyCode::Y => Kikod::Y,
        VirtualKeyCode::Z => Kikod::Z,
        VirtualKeyCode::Escape => Kikod::Escape,
        VirtualKeyCode::F1 => Kikod::F1,
        VirtualKeyCode::F2 => Kikod::F2,
        VirtualKeyCode::F3 => Kikod::F3,
        VirtualKeyCode::F4 => Kikod::F4,
        VirtualKeyCode::F5 => Kikod::F5,
        VirtualKeyCode::F6 => Kikod::F6,
        VirtualKeyCode::F7 => Kikod::F7,
        VirtualKeyCode::F8 => Kikod::F8,
        VirtualKeyCode::F9 => Kikod::F9,
        VirtualKeyCode::F10 => Kikod::F10,
        VirtualKeyCode::F11 => Kikod::F11,
        VirtualKeyCode::F12 => Kikod::F12,
        VirtualKeyCode::F13 => Kikod::F13,
        VirtualKeyCode::F14 => Kikod::F14,
        VirtualKeyCode::F15 => Kikod::F15,
        VirtualKeyCode::F16 => Kikod::F16,
        VirtualKeyCode::F17 => Kikod::F17,
        VirtualKeyCode::F18 => Kikod::F18,
        VirtualKeyCode::F19 => Kikod::F19,
        VirtualKeyCode::F20 => Kikod::F20,
        VirtualKeyCode::F21 => Kikod::F21,
        VirtualKeyCode::F22 => Kikod::F22,
        VirtualKeyCode::F23 => Kikod::F23,
        VirtualKeyCode::F24 => Kikod::F24,
        VirtualKeyCode::Snapshot => Kikod::Snapshot,
        VirtualKeyCode::Scroll => Kikod::Scroll,
        VirtualKeyCode::Pause => Kikod::Pause,
        VirtualKeyCode::Insert => Kikod::Insert,
        VirtualKeyCode::Home => Kikod::Home,
        VirtualKeyCode::Delete => Kikod::Delete,
        VirtualKeyCode::End => Kikod::End,
        VirtualKeyCode::PageDown => Kikod::PageDown,
        VirtualKeyCode::PageUp => Kikod::PageUp,
        VirtualKeyCode::Left => Kikod::Left,
        VirtualKeyCode::Up => Kikod::Up,
        VirtualKeyCode::Right => Kikod::Right,
        VirtualKeyCode::Down => Kikod::Down,
        VirtualKeyCode::Back => Kikod::Back,
        VirtualKeyCode::Return => Kikod::Return,
        VirtualKeyCode::Space => Kikod::Space,
        VirtualKeyCode::Compose => Kikod::Compose,
        VirtualKeyCode::Caret => Kikod::Caret,
        VirtualKeyCode::Numlock => Kikod::Numlock,
        VirtualKeyCode::Numpad0 => Kikod::Numpad0,
        VirtualKeyCode::Numpad1 => Kikod::Numpad1,
        VirtualKeyCode::Numpad2 => Kikod::Numpad2,
        VirtualKeyCode::Numpad3 => Kikod::Numpad3,
        VirtualKeyCode::Numpad4 => Kikod::Numpad4,
        VirtualKeyCode::Numpad5 => Kikod::Numpad5,
        VirtualKeyCode::Numpad6 => Kikod::Numpad6,
        VirtualKeyCode::Numpad7 => Kikod::Numpad7,
        VirtualKeyCode::Numpad8 => Kikod::Numpad8,
        VirtualKeyCode::Numpad9 => Kikod::Numpad9,
        VirtualKeyCode::NumpadAdd => Kikod::NumpadAdd,
        VirtualKeyCode::NumpadDivide => Kikod::NumpadDivide,
        VirtualKeyCode::NumpadDecimal => Kikod::NumpadDecimal,
        VirtualKeyCode::NumpadComma => Kikod::NumpadComma,
        VirtualKeyCode::NumpadEnter => Kikod::NumpadEnter,
        VirtualKeyCode::NumpadEquals => Kikod::NumpadEquals,
        VirtualKeyCode::NumpadMultiply => Kikod::NumpadMultiply,
        VirtualKeyCode::NumpadSubtract => Kikod::NumpadSubtract,
        VirtualKeyCode::AbntC1 => Kikod::AbntC1,
        VirtualKeyCode::AbntC2 => Kikod::AbntC2,
        VirtualKeyCode::Apostrophe => Kikod::Apostrophe,
        VirtualKeyCode::Apps => Kikod::Apps,
        VirtualKeyCode::Asterisk => Kikod::Asterisk,
        VirtualKeyCode::At => Kikod::At,
        VirtualKeyCode::Ax => Kikod::Ax,
        VirtualKeyCode::Backslash => Kikod::Backslash,
        VirtualKeyCode::Calculator => Kikod::Calculator,
        VirtualKeyCode::Capital => Kikod::Capital,
        VirtualKeyCode::Colon => Kikod::Colon,
        VirtualKeyCode::Comma => Kikod::Comma,
        VirtualKeyCode::Convert => Kikod::Convert,
        VirtualKeyCode::Equals => Kikod::Equals,
        VirtualKeyCode::Grave => Kikod::Grave,
        VirtualKeyCode::Kana => Kikod::Kana,
        VirtualKeyCode::Kanji => Kikod::Kanji,
        VirtualKeyCode::LAlt => Kikod::LAlt,
        VirtualKeyCode::LBracket => Kikod::LBracket,
        VirtualKeyCode::LControl => Kikod::LControl,
        VirtualKeyCode::LShift => Kikod::LShift,
        VirtualKeyCode::LWin => Kikod::LWin,
        VirtualKeyCode::Mail => Kikod::Mail,
        VirtualKeyCode::MediaSelect => Kikod::MediaSelect,
        VirtualKeyCode::MediaStop => Kikod::MediaStop,
        VirtualKeyCode::Minus => Kikod::Minus,
        VirtualKeyCode::Mute => Kikod::Mute,
        VirtualKeyCode::MyComputer => Kikod::MyComputer,
        VirtualKeyCode::NavigateForward => Kikod::NavigateForward,
        VirtualKeyCode::NavigateBackward => Kikod::NavigateBackward,
        VirtualKeyCode::NextTrack => Kikod::NextTrack,
        VirtualKeyCode::NoConvert => Kikod::NoConvert,
        VirtualKeyCode::OEM102 => Kikod::OEM102,
        VirtualKeyCode::Period => Kikod::Period,
        VirtualKeyCode::PlayPause => Kikod::PlayPause,
        VirtualKeyCode::Plus => Kikod::Plus,
        VirtualKeyCode::Power => Kikod::Power,
        VirtualKeyCode::PrevTrack => Kikod::PrevTrack,
        VirtualKeyCode::RAlt => Kikod::RAlt,
        VirtualKeyCode::RBracket => Kikod::RBracket,
        VirtualKeyCode::RControl => Kikod::RControl,
        VirtualKeyCode::RShift => Kikod::RShift,
        VirtualKeyCode::RWin => Kikod::RWin,
        VirtualKeyCode::Semicolon => Kikod::Semicolon,
        VirtualKeyCode::Slash => Kikod::Slash,
        VirtualKeyCode::Sleep => Kikod::Sleep,
        VirtualKeyCode::Stop => Kikod::Stop,
        VirtualKeyCode::Sysrq => Kikod::Sysrq,
        VirtualKeyCode::Tab => Kikod::Tab,
        VirtualKeyCode::Underline => Kikod::Underline,
        VirtualKeyCode::Unlabeled => Kikod::Unlabeled,
        VirtualKeyCode::VolumeDown => Kikod::VolumeDown,
        VirtualKeyCode::VolumeUp => Kikod::VolumeUp,
        VirtualKeyCode::Wake => Kikod::Wake,
        VirtualKeyCode::WebBack => Kikod::WebBack,
        VirtualKeyCode::WebFavorites => Kikod::WebFavorites,
        VirtualKeyCode::WebForward => Kikod::WebForward,
        VirtualKeyCode::WebHome => Kikod::WebHome,
        VirtualKeyCode::WebRefresh => Kikod::WebRefresh,
        VirtualKeyCode::WebSearch => Kikod::WebSearch,
        VirtualKeyCode::WebStop => Kikod::WebStop,
        VirtualKeyCode::Yen => Kikod::Yen,
        VirtualKeyCode::Copy => Kikod::Copy,
        VirtualKeyCode::Paste => Kikod::Paste,
        VirtualKeyCode::Cut => Kikod::Cut,
        _ => Kikod::Unknown,
    }
}

/// Example struct holds references to wgpu resources and frame persistent data
struct Example {
    //particle_bind_groups: Vec<wgpu::BindGroup>,
    bind_group_lines: wgpu::BindGroup,
    bind_group_ui: wgpu::BindGroup,
    //particle_buffers: Vec<wgpu::Buffer>,
    vertices_buffers: Vec<wgpu::Buffer>,
    //compute_pipeline: wgpu::ComputePipeline,
    render_pipeline_lines: wgpu::RenderPipeline,
    render_pipeline_ui: wgpu::RenderPipeline,
    //work_group_count: u32,
    frame_num: usize,
    //particles: Particles,
    sliding_renderer: SlidingRenderer,
    index_buf: wgpu::Buffer,
    uniform_buf: wgpu::Buffer,
    #[allow(unused)]
    impl_params: MyParams,
}

fn create_lines_render_pipeline(
    device: &wgpu::Device,
    sconfig: &wgpu::SurfaceConfiguration,
    _adapter: &wgpu::Adapter,
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    //let vs_module = device.create_shader_module(&wgpu::include_spirv!("shaders/shader.vert.spv"));
    //let fs_module = device.create_shader_module(&wgpu::include_spirv!("shaders/shader.frag.spv"));

    println!("create_lines_render_pipeline");

    // Create pipeline layout
    let bind_group_lines_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            }],
        });

    println!("create_lines_render_pipeline shader");
    let wgsl_lines = &include_str_or_read_at_runtime!("shaders/lines4.wgsl");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&kwasm::fix_webgl_color(wgsl_lines))),
    });

    let vertex_buffers = [wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<PosColVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Sint16x2, 1 => Unorm8x4],
    }];
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_lines_layout],
        push_constant_ranges: &[],
    });

    println!("create_lines_render_pipeline render_pipeline");

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        multiview: None,
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: sconfig.format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        operation: wgpu::BlendOperation::Add,
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Max,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Cw,
            //cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
    });

    println!("create_lines_render_pipeline end");

    // let bind_group_lines_layout =
    //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    //         label: None,
    //         entries: &[wgpu::BindGroupLayoutEntry {
    //             binding: 0,
    //             visibility: wgpu::ShaderStage::VERTEX,
    //             ty: wgpu::BindingType::Buffer {
    //                 ty: wgpu::BufferBindingType::Uniform,
    //                 has_dynamic_offset: false,
    //                 min_binding_size: wgpu::BufferSize::new(64),
    //             },
    //             count: None,
    //         }],
    //     });

    // let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    //     label: Some("render pipe lines layout"),
    //     bind_group_layouts: &[&bind_group_lines_layout],
    //     push_constant_ranges: &[],
    // });

    // let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    //     layout: Some(&render_pipeline_layout),
    //     vertex_stage: wgpu::ProgrammableStageDescriptor {
    //         module: &vs_module,
    //         entry_point: "main",
    //     },
    //     fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
    //         module: &fs_module,
    //         entry_point: "main",
    //     }),
    //     rasterization_state: Some(wgpu::RasterizationStateDescriptor {
    //         front_face: wgpu::FrontFace::Ccw,
    //         cull_mode: wgpu::CullMode::None,
    //         ..Default::default()
    //     }),
    //     primitive_topology: wgpu::PrimitiveTopology::TriangleList,
    //     color_states: &[wgpu::ColorStateDescriptor {
    //         format: sc_desc.format,
    //         color_blend: wgpu::BlendDescriptor {
    //             operation: wgpu::BlendOperation::Add,
    //             src_factor: wgpu::BlendFactor::SrcAlpha,
    //             dst_factor: wgpu::BlendFactor::One,
    //         },
    //         alpha_blend: wgpu::BlendDescriptor::REPLACE,
    //         write_mask: wgpu::ColorWrite::ALL,
    //     }],
    //     depth_stencil_state: None,
    //     vertex_state: wgpu::VertexStateDescriptor {
    //         index_format: Some(wgpu::IndexFormat::Uint16),
    //         vertex_buffers: &[wgpu::VertexBufferDescriptor {
    //             stride: 2 * 2 + 1 * 4,
    //             step_mode: wgpu::InputStepMode::Vertex,
    //             attributes: &wgpu::vertex_attr_array![0 => Short2, 1 => Uchar4Norm],
    //         }],
    //     },
    //     sample_count: 1,
    //     sample_mask: !0,
    //     alpha_to_coverage_enabled: false,
    //     label: Some("render pipe lines"),
    // });

    // let mut flags = wgpu::ShaderFlags::VALIDATION;
    // let draw_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    //     label: None,
    //     source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/linest.wgsl"))),
    //     flags,
    // });

    // -----------------------------------------------------
    // let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    //     label: None,
    //     layout: Some(&render_pipeline_layout),

    //     vertex: wgpu::VertexState {
    //         module: &vs_module,
    //         entry_point: "main",
    //         buffers: &[
    //             wgpu::VertexBufferLayout {
    //                 array_stride: std::mem::size_of::<PosColVertex>() as wgpu::BufferAddress,
    //                 step_mode: wgpu::InputStepMode::Vertex,
    //                 attributes: &wgpu::vertex_attr_array![0 => Sint16x2, 1 => Unorm8x4],
    //                 // attributes: &[
    //                 //     wgpu::VertexAttribute {
    //                 //         format: wgpu::VertexFormat::Sint16x2,
    //                 //         offset: 0,
    //                 //         shader_location: 0,
    //                 //     },
    //                 //     wgpu::VertexAttribute {
    //                 //         format: wgpu::VertexFormat::Unorm8x4,
    //                 //         offset: 2 * 2,
    //                 //         shader_location: 1,
    //                 //     },
    //                 // ],
    //             },

    //         ],
    //     },
    //     fragment: Some(wgpu::FragmentState {
    //         module: &fs_module,
    //         entry_point: "main",
    //         // targets: &[sc_desc.format.into()],

    //         targets: &[wgpu::ColorTargetState {
    //             format: sc_desc.format,
    //             blend: Some(wgpu::BlendState {
    //                 color: wgpu::BlendComponent {
    //                     operation: wgpu::BlendOperation::Add,
    //                     src_factor: wgpu::BlendFactor::SrcAlpha,
    //                     dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
    //                 },
    //                 alpha: wgpu::BlendComponent::REPLACE,
    //             }),
    //             //alpha_blend: wgpu::BlendState::REPLACE,
    //             write_mask: wgpu::ColorWrite::ALL,
    //         }],
    //     }),
    //     primitive: wgpu::PrimitiveState::default(),
    //     depth_stencil: None,
    //     multisample: wgpu::MultisampleState::default(),
    // });

    (render_pipeline, bind_group_lines_layout)
}

fn create_ui_render_pipeline(
    device: &wgpu::Device,
    sconfig: &wgpu::SurfaceConfiguration,
    _adapter: &wgpu::Adapter,
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    println!("create_ui_render_pipeline");
    // let vs_module = device.create_shader_module(&wgpu::include_spirv!("shaders/ui.vert.spv"));
    // let fs_module = device.create_shader_module(&wgpu::include_spirv!("shaders/ui.frag.spv"));

    // Create pipeline layout
    let bind_group_lines_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

    println!("create_ui_render_pipeline shader");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&kwasm::fix_webgl_color(
            &include_str_or_read_at_runtime!("shaders/ui2.wgsl"),
        ))),
    });

    let vertex_buffers = [wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<PosColTexVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Unorm8x4, 2 => Float32x2],
    }];
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_lines_layout],
        push_constant_ranges: &[],
    });

    println!("create_ui_render_pipeline render_pipeline");

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        multiview: None,
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: sconfig.format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        operation: wgpu::BlendOperation::Add,
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    },
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Cw,
            //cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
    });

    // let bind_group_lines_layout =
    //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    //         label: None,
    //         entries: &[
    //             wgpu::BindGroupLayoutEntry {
    //                 binding: 0,
    //                 visibility: wgpu::ShaderStage::VERTEX,
    //                 ty: wgpu::BindingType::Buffer {
    //                     ty: wgpu::BufferBindingType::Uniform,
    //                     has_dynamic_offset: false,
    //                     min_binding_size: wgpu::BufferSize::new(64),
    //                 },
    //                 count: None,
    //             },
    //             wgpu::BindGroupLayoutEntry {
    //                 binding: 1,
    //                 visibility: wgpu::ShaderStage::FRAGMENT,
    //                 ty: wgpu::BindingType::Texture {
    //                     multisampled: false,
    //                     sample_type: wgpu::TextureSampleType::Float { filterable: true },
    //                     view_dimension: wgpu::TextureViewDimension::D2,
    //                 },
    //                 count: None,
    //             },
    //             wgpu::BindGroupLayoutEntry {
    //                 binding: 2,
    //                 visibility: wgpu::ShaderStage::FRAGMENT,
    //                 ty: wgpu::BindingType::Sampler {
    //                     comparison: false,
    //                     filtering: true,
    //                 },
    //                 count: None,
    //             },
    //         ],
    //     });

    // let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    //     label: Some("render pipe ui layout"),
    //     bind_group_layouts: &[&bind_group_lines_layout],
    //     push_constant_ranges: &[],
    // });

    // ------------------

    // let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    //     layout: Some(&render_pipeline_layout),
    //     vertex_stage: wgpu::ProgrammableStageDescriptor {
    //         module: &vs_module,
    //         entry_point: "main",
    //     },
    //     fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
    //         module: &fs_module,
    //         entry_point: "main",
    //     }),
    //     rasterization_state: Some(wgpu::RasterizationStateDescriptor {
    //         front_face: wgpu::FrontFace::Ccw,
    //         cull_mode: wgpu::CullMode::None,
    //         ..Default::default()
    //     }),
    //     primitive_topology: wgpu::PrimitiveTopology::TriangleList,
    //     color_states: &[wgpu::ColorStateDescriptor {
    //         format: sc_desc.format,
    //         // color_blend: wgpu::BlendDescriptor {
    //         //     operation: wgpu::BlendOperation::Add,
    //         //     src_factor: wgpu::BlendFactor::SrcAlpha,
    //         //     dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
    //         // },
    //         color_blend: wgpu::BlendDescriptor::REPLACE,
    //         alpha_blend: wgpu::BlendDescriptor::REPLACE,
    //         write_mask: wgpu::ColorWrite::ALL,
    //     }],
    //     depth_stencil_state: None,
    //     vertex_state: wgpu::VertexStateDescriptor {
    //         index_format: None, //Some(wgpu::IndexFormat::Uint16),
    //         vertex_buffers: &[wgpu::VertexBufferDescriptor {
    //             stride: (2 + 1 + 2) * 4,
    //             step_mode: wgpu::InputStepMode::Vertex,
    //             attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Uchar4Norm, 2 => Float2],
    //         }],
    //     },
    //     sample_count: 1,
    //     sample_mask: !0,
    //     alpha_to_coverage_enabled: false,
    //     label: Some("render ui pipe"),
    // });

    // let mut flags = wgpu::ShaderFlags::VALIDATION;
    // match adapter.get_info().backend {
    //     wgpu::Backend::Metal | wgpu::Backend::Vulkan | wgpu::Backend::Gl => {
    //         flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION
    //     }
    //     _ => (), //TODO
    // }
    // // let draw_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    // //     label: None,
    // //     source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/ui.wgsl"))),
    // //     flags,
    // // });

    // let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    //     label: None,
    //     layout: Some(&render_pipeline_layout),
    //     vertex: wgpu::VertexState {
    //         module: &vs_module,
    //         entry_point: "main",
    //         buffers: &[
    //             wgpu::VertexBufferLayout {
    //                 array_stride: std::mem::size_of::<PosColTexVertex>() as wgpu::BufferAddress,
    //                 step_mode: wgpu::InputStepMode::Vertex,
    //                 attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Unorm8x4, 2 => Float32x2],
    //                 // attributes: &[
    //                 //     wgpu::VertexAttribute {
    //                 //         format: wgpu::VertexFormat::Float32x2,
    //                 //         offset: 0,
    //                 //         shader_location: 0,
    //                 //     },
    //                 //     wgpu::VertexAttribute {
    //                 //         format: wgpu::VertexFormat::Unorm8x4,
    //                 //         offset: 4 * 2,
    //                 //         shader_location: 1,
    //                 //     },
    //                 //     wgpu::VertexAttribute {
    //                 //         format: wgpu::VertexFormat::Float32x2,
    //                 //         offset: 4 * 3,
    //                 //         shader_location: 2,
    //                 //     },
    //                 // ],
    //             },

    //         ],
    //     },
    //     fragment: Some(wgpu::FragmentState {
    //         module: &fs_module,
    //         entry_point: "main",
    //         targets: &[sc_desc.format.into()],
    //     }),
    //     primitive: wgpu::PrimitiveState::default(),
    //     depth_stencil: None,
    //     multisample: wgpu::MultisampleState::default(),
    // });
    (render_pipeline, bind_group_lines_layout)
}

// fn include_str_or_read_at_runtime(filename: &'static str) -> String {
// }

impl Example {
    fn generate_matrix(dx: f32, dy: f32) -> cgmath::Matrix4<f32> {
        println!("generate_matrix: dx{} dy{}", dx, dy);
        let mx_projection = cgmath::ortho(0.0, dx, dy, 0.0, -1.0, 1.0);

        let mx_correction = framework::OPENGL_TO_WGPU_MATRIX;
        mx_correction * mx_projection
    }
}
#[derive(Clone)]
pub struct MyParams {
    #[cfg(not(target_arch = "wasm32"))]
    audio_device: Option<String>,
}

fn create_indices() -> Vec<u16> {
    let mut index_data = Vec::new();

    for i in 0..(0xFFFF / 4) {
        index_data.push(i * 4 + 0);
        index_data.push(i * 4 + 1);
        index_data.push(i * 4 + 3);

        index_data.push(i * 4 + 3);
        index_data.push(i * 4 + 2);
        index_data.push(i * 4 + 1);
    }

    index_data
}


// fn nonempty_vertex_buffer(
//     drop_defer: &RefCell<Option<wgpu::Buffer>>,
//     device: &wgpu::Device,
//     label: &str,
//     content: &[u8],
// ) -> () {
//     if content.len() == 0 {
//         drop_defer.replace(None);
//     } else {
//         drop_defer.replace(Some(device.create_buffer_init(
//             &wgpu::util::BufferInitDescriptor {
//                 label: Some(label),
//                 contents: &content,
//                 usage: wgpu::BufferUsages::VERTEX,
//             },
//         )));

//         //drop_defer.last()
//     }
// }

impl framework::Example<MyParams> for Example {
    fn required_limits() -> wgpu::Limits {
        wgpu::Limits::downlevel_defaults()
    }

    fn required_downlevel_capabilities() -> wgpu::DownlevelCapabilities {
        wgpu::DownlevelCapabilities {
            //flags: wgpu::DownlevelFlags::COMPUTE_SHADERS,
            flags: wgpu::DownlevelFlags::empty(),
            ..Default::default()
        }
    }

    /// constructs initial instance of Example struct
    fn init(
        sconfig: &wgpu::SurfaceConfiguration,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        impl_params: MyParams,
    ) -> Self {
        // load (and compile) shaders and create shader modules

        // buffer for simulation parameters uniform
        crate::spectrumapp::kwasm::debug_wasm_mem("Example init");



        crate::spectrumapp::kwasm::debug_wasm_mem("generate_matrix");
        let mx_total = Self::generate_matrix(sconfig.width as f32, sconfig.height as f32);
        let mx_ref: &[f32; 16] = mx_total.as_ref();

        crate::spectrumapp::kwasm::debug_wasm_mem("uniform_buf");
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(mx_ref),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        crate::spectrumapp::kwasm::debug_wasm_mem("create_indices");
        let index_data = create_indices();
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });

        crate::spectrumapp::kwasm::debug_wasm_mem("texture_view");
        // Create the texture
        let texture_view = {
            let size = 256u32;
            //let texels = create_texels(size as usize);
            let texels = &include_bytes!("ascii.raw")[..];
            let texture_extent = wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: texture_extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            });
            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            queue.write_texture(
                texture.as_image_copy(),
                &texels,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(NonZeroU32::new(4 * size).unwrap()),
                    rows_per_image: None,
                },
                texture_extent,
            );
            texture_view
        };
        // Create other resources
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // create compute bind layout group and compute pipeline layout

        /*
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            min_binding_size: wgpu::BufferSize::new(sim_param_data.len() as _),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            min_binding_size: wgpu::BufferSize::new((NUM_PARTICLES * 16) as _),
                            readonly: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            min_binding_size: wgpu::BufferSize::new((NUM_PARTICLES * 16) as _),
                            readonly: false,
                        },
                        count: None,
                    },
                ],
                label: None,
            });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });
            */

        // create render pipeline with empty bind group layout

        let (render_pipeline_lines, bind_group_lines_layout) =
            create_lines_render_pipeline(device, sconfig, adapter);
        let (render_pipeline_ui, bind_group_ui_layout) =
            create_ui_render_pipeline(device, sconfig, adapter);

        // Create bind group
        let bind_group_lines = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_lines_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
            label: None,
        });

        // Create bind group
        let bind_group_ui = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_ui_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: None,
        });

        // create compute pipeline
        // let boids_module =
        //     device.create_shader_module(&wgpu::include_spirv!("shaders/boids.comp.spv"));

        /*
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            compute_stage: wgpu::ProgrammableStageDescriptor {
                module: &boids_module,
                entry_point: "main",
            },
        });
        */

        // buffer for the three 2d triangle vertices of each instance

        //let vertex_buffer_data = [-0.07f32, -0.02, 0.01, -0.02, 0.00, 0.02];

        let mut vertices_buffers = Vec::new();
        for _i in 0..2 {
            let vertices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Vertex Buffer"),
                size: 1024 * 1024 * 16,
                mapped_at_creation: false,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            vertices_buffers.push(vertices_buffer);
        }

        // buffer for all particles data of type [(posx,posy,velx,vely),...]

        // let mut initial_particle_data = vec![0.0f32; (4 * NUM_PARTICLES) as usize];
        // for particle_instance_chunk in initial_particle_data.chunks_mut(4) {
        //     particle_instance_chunk[0] = 2.0 * (rand::random::<f32>() - 0.5); // posx
        //     particle_instance_chunk[1] = 2.0 * (rand::random::<f32>() - 0.5); // posy
        //     particle_instance_chunk[2] = 2.0 * (rand::random::<f32>() - 0.5) * 0.1; // velx
        //     particle_instance_chunk[3] = 2.0 * (rand::random::<f32>() - 0.5) * 0.1;
        //     // vely
        // }

        // creates two buffers of particle data each of size NUM_PARTICLES
        // the two buffers alternate as dst and src for each frame

        // let mut particle_buffers = Vec::<wgpu::Buffer>::new();
        // let mut particle_bind_groups = Vec::<wgpu::BindGroup>::new();
        // for i in 0..2 {
        //     particle_buffers.push(
        //         device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //             label: Some(&format!("Particle Buffer {}", i)),
        //             contents: bytemuck::cast_slice(&initial_particle_data),
        //             usage: wgpu::BufferUsage::VERTEX
        //                 | wgpu::BufferUsage::STORAGE
        //                 | wgpu::BufferUsage::COPY_DST,
        //         }),
        //     );
        // }

        // create two bind groups, one for each buffer as the src
        // where the alternate buffer is used as the dst

        /*
        for i in 0..2 {
            particle_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sim_param_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particle_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: particle_buffers[(i + 1) % 2].as_entire_binding(), // bind to opposite buffer
                    },
                ],
                label: None,
            }));
        }*/

        // calculates number of work groups from PARTICLES_PER_GROUP constant
        // let work_group_count =
        //     ((NUM_PARTICLES as f32) / (PARTICLES_PER_GROUP as f32)).ceil() as u32;

        // returns Example struct and No encoder commands

        let mut sliding_renderer = SlidingRenderer::new(&impl_params);
        sliding_renderer.init();
        sliding_renderer.on_resize(sconfig.width, sconfig.height);
        Example {
            //particle_bind_groups,
            bind_group_lines,
            bind_group_ui,
            //particle_buffers,
            vertices_buffers,
            //compute_pipeline,
            render_pipeline_lines,
            render_pipeline_ui,
            //work_group_count,
            frame_num: 0,
            //particles: Particles::new(),
            sliding_renderer,
            index_buf,
            uniform_buf,
            impl_params,
        }
    }

    /// update is called for any WindowEvent not handled by the framework
    fn update(&mut self, event: winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::Focused(false) => {}
            winit::event::WindowEvent::Focused(true) => {}
            winit::event::WindowEvent::CloseRequested => {}
            winit::event::WindowEvent::KeyboardInput { input, .. } => {

                if input.state == winit::event::ElementState::Pressed {
                    match input.virtual_keycode {
                        Some(key) => match key {
                            VirtualKeyCode::NumpadAdd
                            | VirtualKeyCode::Equals
                            | VirtualKeyCode::Plus => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.on_mouse_wheel(1.0));
                            }
                            VirtualKeyCode::NumpadSubtract | VirtualKeyCode::Minus => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.on_mouse_wheel(-1.0));
                            }
                            VirtualKeyCode::Up => {
                                self.sliding_renderer.on_resolution_scale(1.0);
                            }
                            VirtualKeyCode::Down => {
                                self.sliding_renderer.on_resolution_scale(-1.0);
                            }
                            VirtualKeyCode::Left => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.move_spectrum(true, 30));
                            }
                            VirtualKeyCode::Right => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.move_spectrum(false, 30));
                            }

                            VirtualKeyCode::Space => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.toggle_pause());
                            }

                            VirtualKeyCode::P => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.toggle_peaks());
                            }
                            VirtualKeyCode::L => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.toggle_logarithmic());
                            }
                            VirtualKeyCode::R => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.measurement_reset());
                            }

                            VirtualKeyCode::S => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.toggle_subtraction_peaks());
                            }
                            VirtualKeyCode::C => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.toggle_colorize());
                            }

                            VirtualKeyCode::O => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.cycle_window_type());
                            }

                            VirtualKeyCode::B => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.change_window_subdivisions(false));
                            }
                            VirtualKeyCode::N => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.change_window_subdivisions(true));
                            }
                            VirtualKeyCode::M => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.cycle_method());
                            }

                            VirtualKeyCode::Z => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.change_collect_freq(false));
                            }
                            VirtualKeyCode::X => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.change_collect_freq(true));
                            }

                            VirtualKeyCode::Period => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.measurement_change_freq(false));
                            }
                            VirtualKeyCode::Comma => {
                                self.sliding_renderer
                                    .spectrum_ui
                                    .as_mut()
                                    .map(|v| v.measurement_change_freq(true));
                            }
                            _ => {}
                        },
                        None => {}
                    };
                }
            }
            winit::event::WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::LineDelta(_dx, dy) => {
                    klog!("scroll  LineDelta {}", dy);
                    self.sliding_renderer
                        .spectrum_ui
                        .as_mut()
                        .map(|v| v.on_mouse_wheel(dy));
                }
                winit::event::MouseScrollDelta::PixelDelta(pos) => {
                    let dy = pos.y as f32 / 100.0;
                    klog!("scroll PixelDelta {:?}", dy);
                    self.sliding_renderer
                        .spectrum_ui
                        .as_mut()
                        .map(|v| v.on_mouse_wheel(dy));
                }
            },
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.sliding_renderer
                    .spectrum_ui
                    .as_mut()
                    .map(|v| v.on_cursor_move(position.into()));
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == winit::event::ElementState::Pressed;
                let left = button == winit::event::MouseButton::Left;
                //if button == winit::event::MouseButton::Left {
                self.sliding_renderer
                    .spectrum_ui
                    .as_mut()
                    .map(|v| v.on_mouse_click(pressed, left));
                //}
            }
            _ => {}
        };
    }

    /// resize is called on WindowEvent::Resized events
    fn resize(
        &mut self,
        sconfig: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.sliding_renderer
            .on_resize(sconfig.width, sconfig.height);

        let mx_total = Self::generate_matrix(sconfig.width as f32, sconfig.height as f32);
        let mx_ref: &[f32; 16] = mx_total.as_ref();
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(mx_ref));
    }

    /// render is called each frame, dispatching compute groups proportional
    ///   a TriangleList draw call for all NUM_PARTICLES at 3 vertices each
    fn render(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _spawner: &framework::Spawner,
    ) {
        // create render pass descriptor
        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("render_pass_descriptor"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        };

        // get command encoder
        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {

            /*

                command_encoder.copy_buffer_to_buffer(
                &temp_buf,
                0,
                &self.renderer.instance_buf,
                0,
                (self.renderer.instances.len() * std::mem::size_of::<Instance>()) as u32,
            );*/
        }

        /*{
            // compute pass
            let mut cpass = command_encoder.begin_compute_pass();
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.particle_bind_groups[self.frame_num % 2], &[]);
            cpass.dispatch(self.work_group_count, 1, 1);
        }*/

        self.sliding_renderer.notify_render_tick();

        //let mut buffers_to_drop: Vec<RefCell<wgpu::Buffer>> = Vec::new();

        // let mut cella = RefCell::new(None);
        // let mut cellb = RefCell::new(None);
        match self.sliding_renderer.spectrum_ui.as_mut() {
            None => {}
            Some(spectrum_ui) => {
                //let spam_bytes = circle_f32(&mut self.particles);

                //let (pos_col_verts, pos_col_tex_verts) = (Vec::new(), Vec::<PosColTexVertex>::new());
                let (pos_col_verts, pos_col_tex_verts) = spectrum_ui.render();
                // if pos_col_verts.len() == 0 {
                //     println!("pos_col_verts.len() == 0");
                // }
                // if pos_col_tex_verts.len() == 0 {
                //     println!("pos_col_tex_verts.len() == 0");
                // }

                let pos_col_bytes: &[u8] = bytemuck::cast_slice(&pos_col_verts.as_slice());
                let pos_col_tex_bytes: &[u8] = bytemuck::cast_slice(&pos_col_tex_verts.as_slice());

                let pos_col_buf = {
                    let buffer = &self.vertices_buffers[0];
                    queue.write_buffer(buffer, 0 as wgpu::BufferAddress, pos_col_bytes);
                    buffer
                };
                let pos_col_tex_buf = {
                    let buffer = &self.vertices_buffers[1];
                    queue.write_buffer(buffer, 0 as wgpu::BufferAddress, pos_col_tex_bytes);
                    buffer
                };

                // let pos_col_buf = {
                //     let buffer = &self.vertices_buffers[0];
                //     let slice = buffer.slice(..);
                //     block_on(slice.map_async(wgpu::MapMode::Write)).unwrap();
                //     slice.get_mapped_range_mut()[..pos_col_bytes.len()]
                //         .copy_from_slice(pos_col_bytes);
                //     buffer.unmap();
                //     buffer
                // };
                // let pos_col_tex_buf = {
                //     let buffer = &self.vertices_buffers[1];
                //     let slice = buffer.slice(..);
                //     block_on(slice.map_async(wgpu::MapMode::Write)).unwrap();
                //     slice.get_mapped_range_mut()[..pos_col_tex_bytes.len()]
                //         .copy_from_slice(pos_col_tex_bytes);
                //     buffer.unmap();
                //     buffer
                // };

                // let pos_col_bytes: &[u8] = bytemuck::cast_slice(&pos_col_verts.as_slice());
                // let pos_col_buf = nonempty_vertex_buffer(&cella, &device, "pos_col_buf", pos_col_bytes);

                // let pos_col_tex_bytes: &[u8] = bytemuck::cast_slice(&pos_col_tex_verts.as_slice());
                // let pos_col_tex_buf =
                //     nonempty_vertex_buffer(&cellb, &device, "pos_col_tex_buf", pos_col_tex_bytes);

                {
                    // render pass
                    let mut rpass = command_encoder.begin_render_pass(&render_pass_descriptor);
                    // render dst particles
                    /*rpass.set_vertex_buffer(
                        0,
                        self.particle_buffers[(self.frame_num + 1) % 2].slice(..),
                    );*/
                    // the three instance-local vertices
                    //rpass.set_vertex_buffer(1, self.vertices_buffer.slice(..));

                    rpass.set_pipeline(&self.render_pipeline_lines);
                    rpass.set_bind_group(0, &self.bind_group_lines, &[]);
                    rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
                    //rpass.draw(0..(pos_col_verts.len() as _), 0..1);
                    //println!("draw_indexed {} {}", 0, (pos_col_verts.len() as i32));
                    //let l = pos_col_verts.len() as i32;

                    draw_chunked(&mut rpass, pos_col_buf, &pos_col_verts);
                    {
                        rpass.set_pipeline(&self.render_pipeline_ui);
                        rpass.set_bind_group(0, &self.bind_group_ui, &[]);
                        rpass.set_vertex_buffer(0, pos_col_tex_buf.slice(..));
                        rpass.draw(0..(pos_col_tex_verts.len() as _), 0..1);
                    }

                    //pos_col_buf.as_ref().map(|wbuf| wbuf.unmap());
                    //pos_col_tex_buf.as_ref().map(|wbuf| wbuf.unmap());
                    //pos_col_buf.map(|wbuf| buffers_to_drop.push(wbuf));
                }
            }
        }

        // update frame count
        self.frame_num += 1;

        // done
        queue.submit(Some(command_encoder.finish()));

        //for wbuf in &buffers_to_drop {
        //cella.borrow().as_ref().map(|wbuf| wbuf.destroy());
        //cellb.borrow().as_ref().map(|wbuf| wbuf.destroy());
        //}
    }
}

fn draw_chunked<'a>(
    rpass: &mut wgpu::RenderPass<'a>,
    pos_col_buf: &'a wgpu::Buffer,
    pos_col_verts: &Vec<PosColVertex>,
) {
    for chunk in pos_col_verts.chunks(0x10000 - 8) {
        let a = chunk.as_ptr() as usize - pos_col_verts.as_ptr() as usize;
        let a = a as u64;
        let b = chunk.len() as u64;
        let n = ((chunk.len() * 6) / 4) as u32;
        rpass.set_vertex_buffer(0, pos_col_buf.slice(a..(a + b)));

        rpass.draw_indexed(0..n, 0, 0..1);
    }
}

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Substring of audio input device to use
    #[arg(short, long)]
    input_audio_device: Option<String>,
}

pub fn main() {
    klog!("spectrumapp::main");

    #[cfg(not(target_arch = "wasm32"))]
    let args = Args::parse();

    //let guard = pprof::ProfilerGuard::new(100).unwrap();

    let params = MyParams {
        #[cfg(not(target_arch = "wasm32"))]
        audio_device: args.input_audio_device,
    };
    framework::run::<MyParams, Example>("sbsdft", params);
}

#[derive(Copy, Clone, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum GraphType {
    Line,
    Peaks,
    Fill,
}

struct SlidingRenderer {
    last_screenx: u32,
    last_screeny: u32,
    render_ticks_passed: i64,
    channels: Option<Vec<Weak<Mutex<SlidingImpl>>>>,

    #[cfg(not(target_arch = "wasm32"))]
    audio_io_bridge: Option<Rc<RefCell<adevice_cpal::SlidingCpal>>>,

    #[cfg(target_arch = "wasm32")]
    audio_io_bridge: Option<Rc<RefCell<adevice_web::AdeviceWeb>>>,

    spectrum_ui: Option<SpectrumUI>,

    #[allow(unused)]
    params: MyParams,
}

impl SlidingRenderer {
    fn new(params: &MyParams) -> Self {
        let renderer = Self {
            last_screenx: 100,
            last_screeny: 100,
            audio_io_bridge: None,
            spectrum_ui: None,
            render_ticks_passed: 0,
            channels: None,
            params: params.clone(),
        };

        renderer
    }

    fn init(&mut self) {}

    fn init_app_ui(&mut self) {
        // #[cfg(target_arch = "wasm32")]
        // if !wasm_rayon::wasm_rayon_started() {
        //     return;
        // }
        kwasm::debug_wasm_mem("init_app_ui");

        let config = SpectrumConfig {
            sample_rate: 24000,
            num_bins: 800,
            min_f: 8.0,
            max_f: 10000.0,
            wave_cycles_resolution: 16.0,
            resolution_low_f_shelf_hz: 50.0,
            subtraction_peaks: false,
        };

        let mut impls = vec![];
        for _ in 0..1 {
            let swdft = ChannelSWDFT::new(&config);
            let mut receiver = None;

            std::mem::swap(
                &mut receiver,
                &mut swdft.collected_spectrums_receiver.lock().unwrap(),
            );
            let sliding_impl = SlidingImpl::DFT(swdft);
            let impl_rc = Arc::new(Mutex::new(sliding_impl));

            let impl_main_thread = SlidingMainThread {
                sliding_rc: impl_rc,
                spectrum_receiver: receiver.unwrap(),
                last_rolling_gain: 1.0,
                collected_spectrums: VecDeque::new(),
            };
            impls.push(impl_main_thread);
        }

        let channels: Vec<Weak<Mutex<SlidingImpl>>> = impls
            .iter()
            .map(|c| Arc::downgrade(&c.sliding_rc))
            .collect();

        let font_atlas = {
            let pixels = include_bytes!("ascii.raw");
            let size = 256;
            let rect = KRect::at_origin(size, size, 0);
            let img = KRGBAImage {
                dx: size,
                dy: size,
                pixels: pixels.to_vec(),
            };
            let mut font_atlas = FontAtlas::new();
            font_atlas.resize_texture(&rect, size, &img);

            font_atlas
        };

        self.spectrum_ui = Some(SpectrumUI::new(config, font_atlas, impls));
        self.channels = Some(channels);

        {
            let dx = self.last_screenx;
            let dy = self.last_screeny;
            self.spectrum_ui.as_mut().map(|v| v.on_resize(dx, dy));
        }
    }

    fn init_audio(&mut self) {
        #[cfg(target_arch = "wasm32")]
        if !wasm_rayon::wasm_rayon_started() {
            return;
        }
        let channels = self.channels.as_ref().unwrap().clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.audio_io_bridge = Some(adevice_cpal::SlidingCpal::new(
                Box::new(SlidingImplReceiver::new(channels)),
                &self.params,
            ));
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.audio_io_bridge = Some(adevice_web::AdeviceWeb::new(Box::new(
                SlidingImplReceiver::new(channels),
            )));
        }

        #[cfg(not(target_arch = "wasm32"))]
        self.audio_io_bridge
            .as_mut()
            .map(|v| v.borrow_mut().start());

        #[cfg(target_arch = "wasm32")]
        self.audio_io_bridge
            .as_mut()
            .map(|v| v.borrow_mut().start().unwrap());
    }

    fn notify_render_tick(&mut self) {
        self.render_ticks_passed += 1;

        if self.render_ticks_passed > 5 && self.spectrum_ui.is_none() {
            self.init_app_ui();
        }

        if self.render_ticks_passed > 6 && self.audio_io_bridge.is_none() {
            self.init_audio();
        }
    }

    fn on_resize(&mut self, dx: u32, dy: u32) {
        self.last_screenx = dx;
        self.last_screeny = dy;
        self.spectrum_ui.as_mut().map(|v| v.on_resize(dx, dy));
    }

    fn on_resolution_scale(&mut self, delta: f32) {
        self.spectrum_ui
            .as_mut()
            .map(|v| v.on_resolution_scale(delta));
    }
}

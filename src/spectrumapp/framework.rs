use std::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
use winit::dpi::PhysicalSize;
use winit::dpi::Size;

use winit::event_loop::EventLoopBuilder;
use winit::{
    event::{self, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

#[rustfmt::skip]
#[allow(unused)]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[allow(dead_code)]
pub fn cast_slice<T>(data: &[T]) -> &[u8] {
    use std::{mem::size_of, slice::from_raw_parts};

    unsafe { from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>()) }
}

#[allow(dead_code)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

#[allow(unused)]
pub enum MyEvent {
    CanvasResize(i64, i64),
}

pub trait Example<P>: 'static + Sized {
    fn optional_features() -> wgpu::Features {
        wgpu::Features::empty()
    }
    fn required_features() -> wgpu::Features {
        wgpu::Features::empty()
    }
    fn required_downlevel_capabilities() -> wgpu::DownlevelCapabilities {
        wgpu::DownlevelCapabilities {
            flags: wgpu::DownlevelFlags::empty(),
            shader_model: wgpu::ShaderModel::Sm5,
            ..wgpu::DownlevelCapabilities::default()
        }
    }
    fn required_limits() -> wgpu::Limits {
        wgpu::Limits::downlevel_webgl2_defaults() // These downlevel limits will allow the code to run on all possible hardware
    }
    fn init(
        config: &wgpu::SurfaceConfiguration,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        params: P,
    ) -> Self;
    fn resize(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
    fn update(&mut self, event: WindowEvent);
    fn render(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        spawner: &Spawner,
    );
}

struct Setup {
    window: winit::window::Window,
    event_loop: EventLoop<MyEvent>,
    instance: wgpu::Instance,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

#[cfg(target_arch = "wasm32")]
/// Parse the query string as returned by `web_sys::window()?.location().search()?` and get a
/// specific key out of it.
pub fn parse_url_query_string<'a>(query: &'a str, search_key: &str) -> Option<&'a str> {
    let query_string = query.strip_prefix('?')?;

    for pair in query_string.split('&') {
        let mut pair = pair.split('=');
        let key = pair.next()?;
        let value = pair.next()?;

        if key == search_key {
            return Some(value);
        }
    }

    None
}

// new one
async fn setup<P, E: Example<P>>(title: &str) -> Setup {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    };

    let event_loop: EventLoop<MyEvent> = EventLoopBuilder::<MyEvent>::with_user_event().build();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title(title);
    #[cfg(windows_OFF)] // TODO
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_no_redirection_bitmap(true);
    }
    let window = builder.build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        let query_string = web_sys::window().unwrap().location().search().unwrap();
        let level: log::Level = parse_url_query_string(&query_string, "RUST_LOG")
            .and_then(|x| x.parse().ok())
            .unwrap_or(log::Level::Error);
        console_log::init_with_level(level).expect("could not initialize logger");
        //std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");

        my_resize_handler(&window, &event_loop);
    }

    log::info!("Initializing the surface...");

    let backend = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);

    let instance = wgpu::Instance::new(backend);
    let (size, surface) = unsafe {
        let size = window.inner_size();
        let surface = instance.create_surface(&window);
        (size, surface)
    };
    let adapter =
        wgpu::util::initialize_adapter_from_env_or_default(&instance, backend, Some(&surface))
            .await
            .expect("No suitable GPU adapters found on the system!");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let adapter_info = adapter.get_info();
        println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    }

    let optional_features = E::optional_features();
    let required_features = E::required_features();
    let adapter_features = adapter.features();
    assert!(
        adapter_features.contains(required_features),
        "Adapter does not support required features for this example: {:?}",
        required_features - adapter_features
    );

    let required_downlevel_capabilities = E::required_downlevel_capabilities();
    let downlevel_capabilities = adapter.get_downlevel_capabilities();
    assert!(
        downlevel_capabilities.shader_model >= required_downlevel_capabilities.shader_model,
        "Adapter does not support the minimum shader model required to run this example: {:?}",
        required_downlevel_capabilities.shader_model
    );
    assert!(
        downlevel_capabilities
            .flags
            .contains(required_downlevel_capabilities.flags),
        "Adapter does not support the downlevel capabilities required to run this example: {:?}",
        required_downlevel_capabilities.flags - downlevel_capabilities.flags
    );

    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the surface.
    let needed_limits = E::required_limits().using_resolution(adapter.limits());

    use crate::klog;
    klog!("limits: {:?}", needed_limits);
    //needed_limits.max_compute_workgroups_per_dimension = 0;

    let trace_dir = std::env::var("WGPU_TRACE");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: (optional_features & adapter_features) | required_features,
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    needed_limits
                },
            },
            trace_dir.ok().as_ref().map(std::path::Path::new),
        )
        .await
        .map_err(|err| {
            klog!("adapter.request_device: {:?}", err);
            err
        })
        .expect("Unable to find a suitable GPU adapter!");

    Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        device,
        queue,
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(unused)]
fn get_window_size_from_screen(window: &web_sys::Window) -> (i64, i64) {
    let screen = window.screen().unwrap();
    let document = window.document().unwrap();
    let document_el = document.document_element().unwrap();

    let ratio = window.device_pixel_ratio();

    //let client_width_height = ' cwh:' + document.documentElement.clientWidth + 'x' + document.documentElement.clientHeight;
    let screen_dx = screen.width().unwrap() as f64;
    let screen_dy = screen.height().unwrap() as f64;
    let dx = screen_dx * ratio;
    let dy = screen_dy * ratio;

    // let dx = document_el.client_width() as i64;
    // let dy = document_el.client_height() as i64;
    let dx = dx as i64;
    let dy = dy as i64;
    (dx, dy)
}

#[cfg(target_arch = "wasm32")]
fn get_window_size_from_div_element(window: &web_sys::Window) -> (i64, i64) {
    //let screen = window.screen().unwrap();
    let document = window.document().unwrap();
    let div = document.get_element_by_id("fullscreendiv").unwrap();

    let ratio = window.device_pixel_ratio();
    // let screen_dx = screen.width().unwrap() as f64;
    // let screen_dy = screen.height().unwrap() as f64;
    // let dx = screen_dx * ratio;
    // let dy = screen_dy * ratio;

    let dx = div.client_width() as f64;
    let dy = div.client_height() as f64;
    let dx = dx * ratio;
    let dy = dy * ratio;
    let dx = dx as i64;
    let dy = dy as i64;

    (dx, dy)
}

#[cfg(target_arch = "wasm32")]
fn my_resize_handler(window: &winit::window::Window, event_loop: &EventLoop<MyEvent>) {
    use winit::event_loop::EventLoopProxy;
    let proxy: EventLoopProxy<MyEvent> = event_loop.create_proxy();
    let resize_getter = move || -> (i64, i64) {
        let window: web_sys::Window = web_sys::window().unwrap();

        get_window_size_from_div_element(&window)
    };

    let (dx_init, dy_init) = resize_getter();
    window.set_inner_size(Size::new(PhysicalSize::new(dx_init as u32, dy_init as u32)));

    let on_resize = move || {
        if crate::spectrumapp::panicked() {
            return;
        }
        use crate::klog;
        let (dx, dy) = resize_getter();
        klog!("on_resize dx{} dy{}", dx, dy);

        let _ = proxy.send_event(MyEvent::CanvasResize(dx, dy));
    };
    on_resize();
    use wasm_bindgen::closure::Closure;
    let resize_callback = Closure::wrap(Box::new(on_resize) as Box<dyn FnMut()>);
    if let Some(window) = web_sys::window() {
        use wasm_bindgen::JsCast;
        //let document = window.document().unwrap();
        //let body = document.body().unwrap();
        //let div = document.get_element_by_id("fullscreendiv").unwrap();
        window
            .add_event_listener_with_callback("resize", resize_callback.as_ref().unchecked_ref())
            .unwrap();
    }
    resize_callback.forget();
}

fn start<P, E: Example<P>>(
    Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        device,
        queue,
    }: Setup,
    myparams: P,
) {
    let spawner = Spawner::new();
    // let mut config = wgpu::SurfaceConfiguration {
    //     usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    //     format: surface.get_preferred_format(&adapter).unwrap(),
    //     width: size.width,
    //     height: size.height,
    //     present_mode: wgpu::PresentMode::Mailbox,
    //     alpha_mode: wgpu::CompositeAlphaMode::Auto,
    // };
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_supported_formats(&adapter)[0],
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface.get_supported_alpha_modes(&adapter)[0],
    };
    crate::spectrumapp::kwasm::debug_wasm_mem("pre surface.configure");
    surface.configure(&device, &config);

    crate::spectrumapp::kwasm::debug_wasm_mem("pre E::init");
    log::info!("Initializing the example...");
    let mut example = E::init(&config, &adapter, &device, &queue, myparams);

    #[cfg(not(target_arch = "wasm32"))]
    let mut last_update_inst = Instant::now();
    #[cfg(not(target_arch = "wasm32"))]
    let mut last_frame_inst = Instant::now();
    #[cfg(not(target_arch = "wasm32"))]
    let (mut frame_count, mut accum_time) = (0, 0.0);

    log::info!("Entering render loop...");
    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter); // force ownership by the closure
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        match event {
            event::Event::UserEvent(MyEvent::CanvasResize(dx, dy)) => {
                window.set_inner_size(Size::new(PhysicalSize::new(dx as u32, dy as u32)));
            }
            event::Event::RedrawEventsCleared => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // Clamp to some max framerate to avoid busy-looping too much
                    // (we might be in wgpu::PresentMode::Mailbox, thus discarding superfluous frames)
                    //
                    // winit has window.current_monitor().video_modes() but that is a list of all full screen video modes.
                    // So without extra dependencies it's a bit tricky to get the max refresh rate we can run the window on.
                    // Therefore we just go with 60fps - sorry 120hz+ folks!
                    let target_frametime = Duration::from_secs_f64(1.0 / 120.0);
                    let time_since_last_frame = last_update_inst.elapsed();
                    if time_since_last_frame >= target_frametime {
                        window.request_redraw();
                        last_update_inst = Instant::now();
                    } else {
                        *control_flow = ControlFlow::WaitUntil(
                            Instant::now() + target_frametime - time_since_last_frame,
                        );
                    }

                    spawner.run_until_stalled();
                }

                #[cfg(target_arch = "wasm32")]
                window.request_redraw();
            }
            event::Event::WindowEvent {
                event:
                    WindowEvent::Resized(size)
                    | WindowEvent::ScaleFactorChanged {
                        new_inner_size: &mut size,
                        ..
                    },
                ..
            } => {
                log::info!("Resizing to {:?}", size);
                config.width = size.width.max(1);
                config.height = size.height.max(1);
                example.resize(&config, &device, &queue);
                surface.configure(&device, &config);
            }
            event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(event::VirtualKeyCode::Escape),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                }
                | WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                #[cfg(not(target_arch = "wasm32"))]
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(event::VirtualKeyCode::R),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    println!("{:#?}", instance.generate_report());
                }
                _ => {
                    example.update(event);
                }
            },
            event::Event::RedrawRequested(_) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    accum_time += last_frame_inst.elapsed().as_secs_f32();
                    last_frame_inst = Instant::now();
                    frame_count += 1;
                    if frame_count == 100 {
                        println!(
                            "Avg frame time {}ms",
                            accum_time * 1000.0 / frame_count as f32
                        );
                        accum_time = 0.0;
                        frame_count = 0;
                    }
                }

                let frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        surface.configure(&device, &config);
                        surface
                            .get_current_texture()
                            .expect("Failed to acquire next surface texture!")
                    }
                };
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                example.render(&view, &device, &queue, &spawner);

                frame.present();
            }
            _ => {}
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub struct Spawner<'a> {
    executor: async_executor::LocalExecutor<'a>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a> Spawner<'a> {
    fn new() -> Self {
        Self {
            executor: async_executor::LocalExecutor::new(),
        }
    }

    #[allow(dead_code)]
    pub fn spawn_local(&self, future: impl Future<Output = ()> + 'a) {
        self.executor.spawn(future).detach();
    }

    fn run_until_stalled(&self) {
        while self.executor.try_tick() {}
    }
}

#[cfg(target_arch = "wasm32")]
pub struct Spawner {}

#[cfg(target_arch = "wasm32")]
impl Spawner {
    fn new() -> Self {
        Self {}
    }

    #[allow(dead_code)]
    pub fn spawn_local(&self, future: impl Future<Output = ()> + 'static) {
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run<P, E: Example<P>>(title: &str, params: P) {
    let setup = pollster::block_on(setup::<P, E>(title));
    start::<P, E>(setup, params);
}

// #[cfg(target_arch = "wasm32")]
// pub fn run<P: 'static, E: Example<P>>(title: &str, params: P) {
//     let title = title.to_owned();
//     wasm_bindgen_futures::spawn_local(async move {
//         let setup = setup::<P, E>(&title).await;
//         start::<P, E>(setup, params);
//     });
// }

#[cfg(target_arch = "wasm32")]
pub fn run<P: 'static, E: Example<P>>(title: &str, params: P) {
    use wasm_bindgen::{prelude::*, JsCast};

    let title = title.to_owned();
    wasm_bindgen_futures::spawn_local(async move {
        let setup = setup::<P, E>(&title).await;
        let start_closure = Closure::once_into_js(move || start::<P, E>(setup, params));

        // make sure to handle JS exceptions thrown inside start.
        // Otherwise wasm_bindgen_futures Queue would break and never handle any tasks again.
        // This is required, because winit uses JS exception for control flow to escape from `run`.
        if let Err(error) = call_catch(&start_closure) {
            let is_control_flow_exception = error.dyn_ref::<js_sys::Error>().map_or(false, |e| {
                e.message().includes("Using exceptions for control flow", 0)
            });

            if !is_control_flow_exception {
                web_sys::console::error_1(&error);
            }
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(catch, js_namespace = Function, js_name = "prototype.call.call")]
            fn call_catch(this: &JsValue) -> Result<(), JsValue>;
        }
    });
}
// This allows treating the framework as a standalone example,
// thus avoiding listing the example names in `Cargo.toml`.
#[allow(dead_code)]
fn main() {

    //console_log!("abc main");
}

use std::{
    error::Error,
    fmt::{Debug, Display},
    iter,
    num::NonZeroU32,
    thread,
    time::{Duration, Instant},
};

use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin_winit::DisplayBuilder;
use winit::{
    dpi::PhysicalSize,
    event::{StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

mod gl;
use gl::Gl;

pub struct Icon<'a> {
    width: u32,
    height: u32,
    rgba: &'a [u8],
}

impl<'a> Icon<'a> {
    pub fn new(width: u32, height: u32, rgba: &'a [u8]) -> Self {
        assert!((width * height * 4) as usize == rgba.len());
        Self {
            width,
            height,
            rgba,
        }
    }
}

pub struct EngineBuilder<'a> {
    width: u32,
    height: u32,

    title: &'a str,

    icon: Option<Icon<'a>>,
}

impl<'a> EngineBuilder<'a> {
    pub fn with_dimensions(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }

    #[inline]
    pub fn icon(mut self, icon: Icon<'a>) -> Self {
        self.icon = Some(icon);
        self
    }

    #[inline]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    #[inline]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn build(self) -> Engine {
        Engine::new(self.width, self.height, self.title, self.icon)
    }
}

impl<'a> Default for EngineBuilder<'a> {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            title: &"Game",
            icon: None,
        }
    }
}

pub struct Engine {
    width: u32,
    height: u32,

    event_loop: Option<EventLoop<()>>,
    window: Window,
    gl_config: Config,

    gl: Option<Gl>,
}

impl Engine {
    fn new(width: u32, height: u32, title: &str, icon: Option<Icon>) -> Self {
        let event_loop = EventLoopBuilder::new().build().unwrap();

        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
            .expect("no monitors available");

        let monitor_size = monitor.size();

        let target_game_radii = calculate_fit_radii(
            width as f32,
            height as f32,
            monitor_size.width as f32,
            monitor_size.height as f32,
            0.2,
        );

        let mut window_builder = WindowBuilder::new()
            .with_title(title)
            .with_window_icon(icon.map(|i| {
                winit::window::Icon::from_rgba(i.rgba.to_vec(), i.width, i.height)
                    .expect("invalid icon")
            }));

        window_builder = window_builder
            .with_inner_size(PhysicalSize::new(target_game_radii.0, target_game_radii.1));

        let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));

        let Ok((Some(window), gl_config)) =
            display_builder.build(&event_loop, ConfigTemplateBuilder::new(), |configs| {
                configs.max_by_key(|config| config.num_samples()).unwrap()
            }) else { panic!("expected there to be a window") };

        Self {
            width,
            height,
            window,
            event_loop: Some(event_loop),
            gl_config,
            gl: None,
        }
    }

    pub fn run<F>(&mut self, handle_frame: F) -> Result<(), Box<dyn Error>>
    where
        F: Fn(&mut Context, &mut [u8]) -> bool,
    {
        let gl = Gl::new(&self.window, &self.gl_config, self.width, self.height);

        self.gl = Some(gl);

        let pixel_buf_size = (self.width * self.height) as usize * 3;
        let mut pixels = Vec::with_capacity(pixel_buf_size);
        pixels.extend(iter::repeat(0).take(pixel_buf_size));

        let frame_nanos = 1_000_000_000 / 30;

        let instant = Instant::now();
        let mut next_frame_time = instant.elapsed().as_millis() + frame_nanos;
        let mut current_frame = 0;
        self.event_loop
            .take()
            .unwrap()
            .run(|event, window_target| {
                use winit::event::Event as E;

                let cur_time = instant.elapsed().as_nanos();
                match event {
                    E::NewEvents(start_cause) => match start_cause {
                        StartCause::Init => {
                            window_target.set_control_flow(ControlFlow::Poll);
                        }
                        _ => (),
                    },
                    E::Resumed => {
                        if self.gl.is_none() {
                            self.gl = Some(Gl::new(
                                &self.window,
                                &self.gl_config,
                                self.width,
                                self.height,
                            ));
                        }
                    }
                    E::Suspended => {
                        println!("suspended");
                        self.gl = None;
                    }
                    E::WindowEvent { event, .. } => match event {
                        WindowEvent::Resized(size) => {
                            if size.width != 0 && size.height != 0 && self.gl.is_some() {
                                self.gl.as_mut().unwrap().window_resize(
                                    NonZeroU32::new(size.width).unwrap(),
                                    NonZeroU32::new(size.height).unwrap(),
                                );
                                // self.width = size.width;
                                // self.height = size.height;

                                // let new_pixel_buf_len = (self.width * self.height) as usize * 3;
                                // if pixels.len() < new_pixel_buf_len
                            }
                        }
                        WindowEvent::CloseRequested => {
                            window_target.exit();
                        }
                        WindowEvent::RedrawRequested => {
                            if let Some(gl) = &mut self.gl {
                                if cur_time >= next_frame_time {
                                    while cur_time >= next_frame_time {
                                        next_frame_time += frame_nanos;

                                        current_frame += 1;

                                        let mut ctx = Context {
                                            width: self.width,
                                            height: self.height,
                                            current_frame,
                                            will_exit: false,
                                        };
                                        handle_frame(&mut ctx, &mut pixels);

                                        if ctx.will_exit {}
                                    }
                                    gl.draw(&mut self.window, &mut pixels);
                                }
                            }
                        }

                        _ => (),
                    },
                    E::AboutToWait => {
                        if cur_time < next_frame_time {
                            thread::sleep(Duration::from_nanos(
                                (next_frame_time - cur_time) as u64,
                            ));
                        }
                        self.window.request_redraw();
                    }
                    _ => (),
                }
            })?;

        Ok(())
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }
}

pub struct Context {
    width: u32,
    height: u32,
    current_frame: u64,

    will_exit: bool,
}

impl Context {
    pub fn exit(&mut self) {
        self.will_exit = true;
    }
    pub fn prevent_exit(&mut self) {
        self.will_exit = false;
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }
    #[inline]
    pub fn current_frame(&self) -> u64 {
        self.current_frame
    }
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    #[inline]
    pub fn will_exit(&self) -> bool {
        self.will_exit
    }
}

fn calculate_bounding_box(
    width: f32,
    height: f32,
    window_width: f32,
    window_height: f32,
) -> (f32, f32, f32, f32) {
    let window_radii = calculate_fit_radii(width, height, window_width, window_height, 0.1);
    let radii = (
        window_radii.0 / window_width,
        window_radii.1 / window_height,
    );
    (-radii.0, -radii.1, radii.0, radii.1)
}

fn calculate_fit_radii(
    width: f32,
    height: f32,
    window_width: f32,
    window_height: f32,
    margin: f32,
) -> (f32, f32) {
    let margin_units = 2.0 * margin * f32::min(window_width, window_height);
    let remaining_space = (window_width - margin_units, window_height - margin_units);
    let scaled = (remaining_space.0 / width, remaining_space.1 / height);
    let fit_scale_fac = f32::min(scaled.0, scaled.1);
    let radii = (width * fit_scale_fac, height * fit_scale_fac);
    radii
}

pub(crate) struct StrError {
    str: &'static str,
}

impl StrError {
    pub fn new(str: &'static str) -> Self {
        Self { str }
    }
}

impl Display for StrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.str)
    }
}
impl Debug for StrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.str)
    }
}

impl Error for StrError {}

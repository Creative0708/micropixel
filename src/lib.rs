use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
};

pub mod audio;

mod platform;
use audio::{ActiveAudio, AudioWrapper};
use platform::{Window, WindowTrait};

use crate::platform::WindowClient;

pub struct Icon {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Icon {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        assert!((width * height * 4) as usize == rgba.len());
        Self {
            width,
            height,
            rgba,
        }
    }
}

pub struct EngineBuilder {
    width: u32,
    height: u32,
    fullscreen: bool,

    title: String,

    icon: Option<Icon>,
}

mod key;
pub use key::Key;

impl EngineBuilder {
    pub fn with_dimensions(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }
    pub fn fullscreen(target_width: u32, target_height: u32) -> Self {
        Self {
            width: target_width,
            height: target_height,
            fullscreen: true,
            ..Default::default()
        }
    }

    #[inline]
    pub fn icon(mut self, icon: Icon) -> Self {
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
    pub fn title(mut self, title: String) -> Self {
        self.title = title;
        self
    }

    pub fn build(self) -> Engine {
        Engine::new(self)
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            fullscreen: false,
            title: String::from("Game"),
            icon: None,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
enum PressedState {
    JustPressed,
    Pressed,
    JustReleased,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

pub struct Engine {
    width: u32,
    height: u32,

    window_width: u32,
    window_height: u32,

    window: Option<Window>,

    audio: Option<ActiveAudio>,

    pixels: Vec<u8>,
}

impl Engine {
    fn new(builder: EngineBuilder) -> Self {
        let EngineBuilder {
            width,
            height,
            title,
            icon,
            fullscreen,
            ..
        } = builder;

        let window = Window::new(width, height, &title, icon, fullscreen);
        let window_size = window.window_dimensions();

        Self {
            width,
            height,

            window_width: window_size.0,
            window_height: window_size.1,

            window: Some(window),

            audio: ActiveAudio::new().unwrap_or_else(|err| panic!("{err:?}")),

            pixels: Vec::new(),
        }
    }

    pub fn run<F>(&mut self, handle_frame: F)
    where
        F: FnMut(&mut Context, AudioWrapper, &mut [u8]) -> (),
    {
        let pixel_buf_size = (self.width * self.height) as usize * 3;
        self.pixels.resize(pixel_buf_size, 0);

        struct WindowRunner<'a, F>
        where
            F: FnMut(&mut Context, AudioWrapper, &mut [u8]) -> (),
        {
            current_frame: u64,

            bounding_box: (f32, f32, f32, f32),

            engine: &'a mut Engine,
            handle_frame: F,

            is_focused: bool,

            mouse_pos: (f32, f32),
            is_mouse_in_window: bool,

            mouse_button_states: HashMap<MouseButton, PressedState>,
            key_states: HashMap<Key, PressedState>,

            will_exit: bool,
        }

        impl<'a, F> WindowClient for WindowRunner<'a, F>
        where
            F: FnMut(&mut Context, AudioWrapper, &mut [u8]) -> (),
        {
            fn handle_event(&mut self, event: platform::WindowEvent) {
                let engine = &mut self.engine;

                match event {
                    platform::WindowEvent::MouseButton { button, pressed } => {
                        self.mouse_button_states.insert(
                            button,
                            if pressed {
                                PressedState::JustPressed
                            } else {
                                PressedState::JustReleased
                            },
                        );
                    }
                    platform::WindowEvent::Key { key, pressed } => {
                        self.key_states.insert(
                            key,
                            if pressed {
                                PressedState::JustPressed
                            } else {
                                PressedState::JustReleased
                            },
                        );
                    }
                    platform::WindowEvent::MouseEnter { entered } => {
                        self.is_mouse_in_window = entered
                    }
                    platform::WindowEvent::MousePos { x, y } => {
                        let bounding_box = self.bounding_box;
                        let half_dimensions = (
                            engine.window_width as f32 * 0.5,
                            engine.window_height as f32 * 0.5,
                        );
                        let (bounding_box_min_corner, bounding_box_dimensions) = (
                            (
                                bounding_box.0 * half_dimensions.0 + half_dimensions.0,
                                bounding_box.1 * half_dimensions.1 + half_dimensions.1,
                            ),
                            (
                                (bounding_box.2 - bounding_box.0) * half_dimensions.0,
                                (bounding_box.3 - bounding_box.1) * half_dimensions.1,
                            ),
                        );
                        self.mouse_pos = (
                            (x as f32 - bounding_box_min_corner.0) / bounding_box_dimensions.0
                                * engine.width as f32,
                            (y as f32 - bounding_box_min_corner.1) / bounding_box_dimensions.1
                                * engine.height as f32,
                        );
                    }
                    platform::WindowEvent::FocusChanged { focused } => self.is_focused = focused,
                    platform::WindowEvent::WindowClose => self.will_exit = true,
                    platform::WindowEvent::WindowResize {
                        width,
                        height,
                        window_width,
                        window_height,
                        new_bounding_box,
                    } => {
                        let engine = &mut self.engine;
                        engine.width = width;
                        engine.height = height;
                        engine.window_width = window_width;
                        engine.window_height = window_height;

                        let pixel_buf_size = (width * height) as usize * 3;
                        engine.pixels.resize(pixel_buf_size, 0);
                        self.bounding_box = new_bounding_box;
                    }
                }
            }

            fn frame(&mut self) -> bool {
                let engine = &mut self.engine;

                let mut ctx = Context {
                    width: engine.width,
                    height: engine.height,
                    current_frame: self.current_frame,

                    mouse_pos: self.mouse_pos,
                    is_mouse_in_window: self.is_mouse_in_window,

                    mouse_button_states: &self.mouse_button_states,

                    key_states: &self.key_states,

                    will_exit: self.will_exit,
                };
                (self.handle_frame)(
                    &mut ctx,
                    AudioWrapper::new(engine.audio.as_mut()),
                    &mut engine.pixels,
                );

                self.current_frame += 1;

                let will_exit = !ctx.will_exit;

                self.key_states
                    .retain(|_, state| *state != PressedState::JustReleased);
                for (_, state) in self.key_states.iter_mut() {
                    if *state == PressedState::JustPressed {
                        *state = PressedState::Pressed;
                    }
                }
                self.mouse_button_states
                    .retain(|_, state| *state != PressedState::JustReleased);
                for (_, state) in self.mouse_button_states.iter_mut() {
                    if *state == PressedState::JustPressed {
                        *state = PressedState::Pressed;
                    }
                }

                will_exit
            }

            fn get_pixels(&self) -> &[u8] {
                &self.engine.pixels
            }

            fn get_bounding_box(&self) -> (f32, f32, f32, f32) {
                self.bounding_box
            }
        }

        let mut window = self.window.take().unwrap();
        window.run(&mut WindowRunner {
            bounding_box: window.current_bounding_box(),
            current_frame: 0,
            engine: self,
            handle_frame,

            is_focused: false,

            mouse_pos: (0.0, 0.0),
            is_mouse_in_window: false,
            mouse_button_states: HashMap::new(),
            key_states: HashMap::new(),

            will_exit: false,
        });
    }

    // fn recalculate_gl(&mut self) {
    //     let gl = self
    //         .gl
    //         .get_or_insert_with(|| Gl::new(&self.window, &self.gl_config, self.width, self.height));
    //     gl.recalculate_gl(
    //         self.window_width,
    //         self.window_height,
    //         self.fullscreen_target_dimensions,
    //     );
    //     if self.fullscreen_target_dimensions.is_some()
    //         && !(self.width == gl.width() && self.height == gl.height())
    //     {
    //         self.width = gl.width();
    //         self.height = gl.height();
    //         self.pixels
    //             .resize((self.width * self.height) as usize * 3, 0);
    //     }
    // }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }
}

pub struct Context<'a> {
    width: u32,
    height: u32,
    current_frame: u64,

    mouse_pos: (f32, f32),
    is_mouse_in_window: bool,

    mouse_button_states: &'a HashMap<MouseButton, PressedState>,

    key_states: &'a HashMap<Key, PressedState>,

    will_exit: bool,
}

impl<'a> Context<'a> {
    #[inline]
    pub fn will_exit(&self) -> bool {
        self.will_exit
    }
    #[inline]
    pub fn exit(&mut self) {
        self.will_exit = true;
    }
    #[inline]
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
    pub fn mouse_x(&self) -> f32 {
        self.mouse_pos.0
    }
    #[inline]
    pub fn mouse_y(&self) -> f32 {
        self.mouse_pos.1
    }
    #[inline]
    pub fn mouse_pos(&self) -> (f32, f32) {
        self.mouse_pos
    }
    #[inline]
    pub fn integer_mouse_pos(&self) -> (i32, i32) {
        (self.mouse_pos.0 as i32, self.mouse_pos.1 as i32)
    }
    #[inline]
    pub fn is_mouse_in_window(&self) -> bool {
        self.is_mouse_in_window
    }
    #[inline]
    pub fn is_mouse_in_game_area(&self) -> bool {
        if !self.is_mouse_in_window() {
            return false;
        }
        let (mouse_x, mouse_y) = self.integer_mouse_pos();
        mouse_x >= 0 && mouse_x < self.width as i32 && mouse_y >= 0 && mouse_y < self.height as i32
    }

    pub fn is_key_pressed(&self, key_code: Key) -> bool {
        self.key_states
            .get(&key_code)
            .map_or(false, |state| *state != PressedState::JustReleased)
    }
    pub fn is_key_just_pressed(&self, key_code: Key) -> bool {
        self.key_states
            .get(&key_code)
            .map_or(false, |state| *state == PressedState::JustPressed)
    }
    pub fn is_key_just_released(&self, key_code: Key) -> bool {
        self.key_states
            .get(&key_code)
            .map_or(false, |state| *state == PressedState::JustReleased)
    }
    #[inline]
    pub fn is_mouse_button_pressed(&self, mouse_button: MouseButton) -> bool {
        self.mouse_button_states
            .get(&mouse_button)
            .map_or(false, |state| *state != PressedState::JustReleased)
    }
    pub fn is_mouse_button_just_pressed(&self, mouse_button: MouseButton) -> bool {
        self.mouse_button_states
            .get(&mouse_button)
            .map_or(false, |state| *state == PressedState::JustPressed)
    }
    pub fn is_mouse_button_just_released(&self, mouse_button: MouseButton) -> bool {
        self.mouse_button_states
            .get(&mouse_button)
            .map_or(false, |state| *state == PressedState::JustReleased)
    }
}

fn calculate_fit_radii(
    width: f32,
    height: f32,
    container_width: f32,
    container_height: f32,
    margin: f32,
) -> (f32, f32) {
    let margin_units = 2.0 * margin * f32::min(container_width, container_height);
    let remaining_space = (
        container_width - margin_units,
        container_height - margin_units,
    );
    let scaled = (remaining_space.0 / width, remaining_space.1 / height);
    let fit_scale_fac = f32::min(scaled.0, scaled.1);
    let radii = (width * fit_scale_fac, height * fit_scale_fac);
    radii
}

pub(crate) fn get_window_size(
    width: u32,
    height: u32,
    monitor_width: u32,
    monitor_height: u32,
) -> (f32, f32) {
    calculate_fit_radii(
        width as f32,
        height as f32,
        monitor_width as f32,
        monitor_height as f32,
        0.2,
    )
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
        f.write_str(self.str)
    }
}
impl Debug for StrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.str)
    }
}

impl Error for StrError {}

use std::{thread, time};

use glfw::{Context, PixelImage};

use crate::platform::{self, WindowClient, WindowEvent};

use super::Gl;

pub struct GLFWWindow {
    glfw: glfw::Glfw,
    window: glfw::PWindow,
    events: glfw::GlfwReceiver<(f64, glfw::WindowEvent)>,

    fullscreen_target_dimensions: Option<(u32, u32)>,

    gl: super::Gl,
}

impl crate::platform::WindowTrait for GLFWWindow {
    fn new(
        width: u32,
        height: u32,
        title: &str,
        icon: Option<crate::platform::Icon>,
        fullscreen: bool,
    ) -> Self {
        let mut glfw = glfw::init(|error, description| {
            glfw::fail_on_errors(error, description);
        })
        .expect("failed to create GLFW instance");

        glfw.window_hint(glfw::WindowHint::ContextVersionMajor(3));
        glfw.window_hint(glfw::WindowHint::ContextVersionMinor(3));
        glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(
            glfw::OpenGlProfileHint::Core,
        ));

        let (mut window, events) = glfw.with_primary_monitor(|glfw, monitor| {
            let monitor = monitor.expect("failed to get the primary monitor");
            let monitor_size = monitor
                .get_video_mode()
                .map_or((480, 360), |mode| (mode.width, mode.height));

            let window_size =
                crate::get_window_size(width, height, monitor_size.0 as u32, monitor_size.1 as u32);

            glfw.create_window(
                window_size.0 as u32,
                window_size.1 as u32,
                title,
                glfw::WindowMode::Windowed,
            )
            .expect("failed to create GLFW window")
        });

        if let Some(icon) = icon {
            let pixels_u8 = icon.rgba;
            let mut pixels = Vec::with_capacity(pixels_u8.len() / 4);
            for [r, g, b, a] in pixels_u8
                .chunks_exact(4)
                .map(|x| TryInto::<[u8; 4]>::try_into(x).unwrap())
            {
                pixels.push(u32::from_be_bytes([r, g, b, a]));
            }
            window.set_icon_from_pixels(vec![PixelImage {
                width: icon.width,
                height: icon.height,
                pixels,
            }]);
        }

        window.set_size_polling(true);
        window.set_close_polling(true);
        window.set_key_polling(true);
        window.set_focus_polling(true);
        window.set_mouse_button_polling(true);
        window.set_cursor_pos_polling(true);
        window.set_cursor_enter_polling(true);

        let mut gl = Gl::new(width, height, |s| window.get_proc_address(s) as _);

        let window_size = window.get_size();

        let fullscreen_target_dimensions = fullscreen.then_some((width, height));

        gl.recalculate_dimensions_and_bounding_box(
            window_size.0 as _,
            window_size.1 as _,
            fullscreen_target_dimensions,
        );

        glfw.set_swap_interval(glfw::SwapInterval::Sync(1));

        Self {
            glfw,
            window,
            events,

            fullscreen_target_dimensions,

            gl,
        }
    }

    fn window_dimensions(&self) -> (u32, u32) {
        let window_size = self.window.get_size();

        (window_size.0 as u32, window_size.1 as u32)
    }

    fn run<T>(&mut self, client: &mut T)
    where
        T: WindowClient,
    {
        let frame_nanos = 1_000_000_000 / 60;

        let instant = time::Instant::now();
        let mut next_frame_time = instant.elapsed().as_millis() + frame_nanos;

        loop {
            self.glfw.poll_events();

            for (_, glfw_event) in glfw::flush_messages(&self.events) {
                use crate::platform::WindowEvent as W;
                use glfw::WindowEvent as E;
                let event = match glfw_event {
                    E::Key(key, _, action, _) => W::Key {
                        key: match super::glfw_key_to_key(key) {
                            Some(key) => key,
                            None => continue,
                        },
                        pressed: match action {
                            glfw::Action::Release => false,
                            glfw::Action::Press => true,
                            glfw::Action::Repeat => continue,
                        },
                    },
                    E::Size(width, height) => {
                        self.gl.recalculate_dimensions_and_bounding_box(
                            width as u32,
                            height as u32,
                            self.fullscreen_target_dimensions,
                        );
                        let (width, height) = self.gl.dimensions();
                        W::WindowResize { width, height }
                    }
                    E::Close => W::WindowClose,
                    E::Focus(focused) => WindowEvent::FocusChanged { focused },
                    E::MouseButton(mouse_button, action, ..) => W::MouseButton {
                        button: match mouse_button {
                            glfw::MouseButtonLeft => platform::MouseButton::Left,
                            glfw::MouseButtonMiddle => platform::MouseButton::Middle,
                            glfw::MouseButtonRight => platform::MouseButton::Right,
                            _ => continue,
                        },
                        pressed: match action {
                            glfw::Action::Release => false,
                            glfw::Action::Press => true,
                            glfw::Action::Repeat => continue,
                        },
                    },
                    E::CursorPos(x, y) => W::MousePos {
                        x: x as u32,
                        y: y as u32,
                    },
                    E::CursorEnter(entered) => W::MouseEnter { entered },
                    E::Scroll(_, _) => todo!(),
                    _ => continue,
                };

                client.handle_event(event);
            }

            let cur_time = instant.elapsed().as_nanos();

            while cur_time >= next_frame_time {
                next_frame_time += frame_nanos;

                if !client.frame() {
                    return;
                }
            }

            if cur_time < next_frame_time {
                self.gl.draw(client.get_pixels());
                self.window.swap_buffers();

                thread::sleep(time::Duration::from_nanos(
                    (next_frame_time - cur_time) as u64,
                ));
            }
        }
    }

    fn current_bounding_box(&self) -> (f32, f32, f32, f32) {
        self.gl.current_bounding_box()
    }
}

impl Drop for GLFWWindow {
    fn drop(&mut self) {
        self.gl.deinit();
    }
}

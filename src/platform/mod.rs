use crate::{Key, MouseButton};

pub trait WindowTrait: Sized {
    fn new(
        width: u32,
        height: u32,
        title: &str,
        icon: Option<crate::Icon>,
        fullscreen: bool,
    ) -> Self;

    fn window_dimensions(&self) -> (u32, u32);

    fn current_bounding_box(&self) -> (f32, f32, f32, f32);

    fn run<T>(&mut self, client: &mut T)
    where
        T: WindowClient;
}

pub(crate) fn calculate_dimensions_and_bounding_box(
    target_width: u32,
    target_height: u32,
    window_width: u32,
    window_height: u32,
    fullscreen: bool,
) -> ((u32, u32), (f32, f32, f32, f32)) {
    let (window_width, window_height) = (window_width as f32, window_height as f32);

    if fullscreen {
        let target_pixel_size = f32::min(
            window_width / target_width as f32,
            window_height / target_height as f32,
        );
        let min_pixel_size = f32::max(
            window_width / target_width as f32,
            window_height / target_height as f32,
        ) * 0.5;
        let pixel_size = f32::max(target_pixel_size, min_pixel_size);

        let width = (window_width / pixel_size).ceil() as u32;
        let height = (window_height / pixel_size).ceil() as u32;
        let radii = (
            width as f32 * pixel_size / window_width,
            height as f32 * pixel_size / window_height,
        );
        ((width, height), (-radii.0, -radii.1, radii.0, radii.1))
    } else {
        let window_radii = crate::calculate_fit_radii(
            target_width as f32,
            target_height as f32,
            window_width,
            window_height,
            0.1,
        );
        let radii = (
            window_radii.0 / window_width,
            window_radii.1 / window_height,
        );
        (
            (target_width, target_height),
            (-radii.0, -radii.1, radii.0, radii.1),
        )
    }
}
pub trait WindowClient: Sized {
    fn handle_event(&mut self, event: WindowEvent);
    fn frame(&mut self) -> bool;
    fn get_bounding_box(&self) -> (f32, f32, f32, f32);
    fn get_pixels(&self) -> &[u8];
}

#[derive(Debug)]
pub enum WindowEvent {
    MouseButton {
        button: MouseButton,
        pressed: bool,
    },
    Key {
        key: Key,
        pressed: bool,
    },
    MouseEnter {
        entered: bool,
    },
    MousePos {
        x: u32,
        y: u32,
    },
    FocusChanged {
        focused: bool,
    },
    WindowClose,
    WindowResize {
        width: u32,
        height: u32,
        window_width: u32,
        window_height: u32,
        new_bounding_box: (f32, f32, f32, f32),
    },
}

mod native;
pub type Window = native::GLFWWindow;

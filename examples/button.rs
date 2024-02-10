#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use micropixel::*;

fn main() {
    let mut engine = EngineBuilder::fullscreen(33, 33)
        .title("Button".into())
        .build();

    let mut beep_channel = None;
    engine.run(move |ctx, mut audio, pixels| {
        let frame = ctx.current_frame();
        if frame == 0 {
            beep_channel = Some(audio.add_synth_channel(Box::new([-1.0, -1.0, 1.0])));
        }

        pixels.fill([16; 3]);

        let (width, height) = ctx.dimensions();

        let mut set_pixel = |x: i32, y: i32, gray: u8| {
            pixels[(x + y * width as i32) as usize] = [gray; 3];
        };

        const BUTTON_RADIUS: i32 = 4;

        let (button_x, button_y) = ((width / 2) as i32, (height / 2) as i32);

        let (mouse_x, mouse_y) = ctx.integer_mouse_pos();
        let button_hovered = (button_x - mouse_x).abs() <= BUTTON_RADIUS
            && (button_y - mouse_y).abs() <= BUTTON_RADIUS;

        let mouse_down = ctx.is_mouse_button_pressed(MouseButton::Left);

        let (button_offset, button_color) = if button_hovered && mouse_down {
            (1, 192)
        } else if button_hovered {
            (-1, 255)
        } else {
            (0, 224)
        };

        for dx in -BUTTON_RADIUS..=BUTTON_RADIUS {
            for dy in -BUTTON_RADIUS..=BUTTON_RADIUS {
                set_pixel(button_x + dx, button_y + dy + button_offset, button_color);
            }
            for dy in BUTTON_RADIUS + button_offset..=BUTTON_RADIUS + 1 {
                set_pixel(button_x + dx, button_y + dy, 128);
            }
        }

        if button_hovered && ctx.is_mouse_button_just_pressed(MouseButton::Left) {
            let channel = audio.get_channel(beep_channel.unwrap());
            let rand = (frame ^ 0xDEADBEEF).overflowing_mul(frame).0;
            let rand = rand.overflowing_mul(rand).0;
            channel.play_pitch(300.0 + (rand % 0xFFFE) as f32 / 512.0);
            channel.volume_sweep(0.0, 0.2);
        }

        if ctx.is_key_just_pressed(Key::Q) || ctx.is_key_just_pressed(Key::Escape) {
            ctx.exit();
        }
    })
}

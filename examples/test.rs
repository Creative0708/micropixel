use micropixel::KeyCode;
use tinyrand::Rand;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = micropixel::EngineBuilder::fullscreen(40, 80)
        .title("Test")
        .build();

    const NOTE_DATA: &[(i32, i16)] = &[
        (-4, -5),
        (-3, -3),
        (-2, 0),
        (-1, -3),
        (0, 4),
        (3, 4),
        (6, 2),
        //
        (12, -5),
        (13, -3),
        (14, 0),
        (15, -3),
        (16, 2),
        (19, 2),
        (22, 0),
        (25, -1),
        (26, -3),
        //
        (28, -5),
        (29, -3),
        (30, 0),
        (31, -3),
        (32, 0),
        (36, 2),
        (38, -1),
        (41, -3),
        (42, -5),
        (46, -5),
        (48, 2),
        (52, 0),
        //
        (64 + -4, -5),
        (64 + -3, -3),
        (64 + -2, 0),
        (64 + -1, -3),
        (64 + 0, 4),
        (64 + 3, 4),
        (64 + 6, 2),
        //
        (64 + 12, -5),
        (64 + 13, -3),
        (64 + 14, 0),
        (64 + 15, -3),
        (64 + 16, 7),
        (64 + 20, -1),
        (64 + 22, 0),
        (64 + 25, -1),
        (64 + 26, -3),
        //
        (64 + 28, -5),
        (64 + 29, -3),
        (64 + 30, 0),
        (64 + 31, -3),
        (64 + 32, 0),
        (64 + 36, 2),
        (64 + 38, -1),
        (64 + 41, -3),
        (64 + 42, -5),
        (64 + 46, -5),
        (64 + 48, 2),
        (64 + 52, 0),
    ];

    let mut synth = 0;
    let mut noise = 0;
    let mut rand = tinyrand::StdRand::default();
    let mut is_random = false;
    engine.run(move |ctx, audio, pixels| {
        let (width, height) = ctx.dimensions();
        let frame = ctx.current_frame();

        {
            if frame == 0 {
                // let mut v = Vec::new();
                // for i in -5..5 {
                //     v.push(i as f32 * 0.2);
                // }
                // for i in (-4..=5).rev() {
                //     v.push(i as f32 * 0.2);
                // }
                // synth = audio.add_synth_channel(v.into_boxed_slice());
                synth = audio.add_synth_channel(Box::new([-1.0, 1.0]));
                noise = audio.add_noise_channel();
            }

            let mut channels = audio.channels();

            if frame == 0 {
                // channels.get(synth).play_note(-24);
                let synth = channels.get(synth);
                synth.play();
            }

            let mut val_iter = NOTE_DATA
                .iter()
                .skip_while(|(x, _)| (frame != (*x + 4) as u64 * 3))
                .map(|(x, y)| (*x, *y));

            if let Some((_, val)) = val_iter.next() {
                let synth = channels.get(synth);

                synth.set_note(val);
                synth.set_volume(1.0);
                synth.volume_sweep(0.0, 1.0);
            }

            if ctx.is_key_just_pressed(KeyCode::ShiftLeft) {
                is_random = true;
                channels.get(noise).play_note(40);
                channels.get(synth).set_channel_volume(0.0);
            } else if ctx.is_key_just_released(KeyCode::ShiftLeft) {
                is_random = false;
                channels.get(noise).stop();
            }
        }
        if is_random {
            pixels.fill_with(|| rand.next_u16() as u8);
        } else {
            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize * 3;
                    if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                        pixels[idx] = 255;
                        pixels[idx + 1] = 255;
                        pixels[idx + 2] = 255;
                    } else {
                        let (r, g, b) = hsv_to_rgb(
                            ((x as f32 + y as f32 + frame as f32 * 0.1) * 20.0 % 256.0) as u8,
                            220,
                            255,
                        );
                        pixels[idx] = r;
                        pixels[idx + 1] = g;
                        pixels[idx + 2] = b;
                    }
                }
            }
        }

        let (mouse_x, mouse_y) = ctx.integer_mouse_pos();

        if ctx.is_mouse_in_game_area() {
            let idx = (mouse_y * width as i32 + mouse_x) as usize * 3;
            pixels[idx..idx + 3].fill(255);
        }

        if ctx.is_key_just_pressed(KeyCode::KeyQ) {
            ctx.exit();
        }
    })?;

    Ok(())
}

fn hsv_to_rgb(h: u8, s: u8, v: u8) -> (u8, u8, u8) {
    let h = h as f32 / 255.0;
    let s = s as f32 / 255.0;
    let v = v as f32 / 255.0;

    let c = v * s;
    let h = h * 6.0;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u8 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        5 => (c, 0.0, x),
        _ => (0.0, 0.0, 0.0),
    };
    let m = v - c;
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

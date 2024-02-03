fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = micropixel::EngineBuilder::with_dimensions(40, 80)
        .title("Test")
        .build();

    engine.run(|ctx, pixels| {
        let (width, height) = ctx.dimensions();
        let frame = ctx.current_frame();

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize * 3;
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

        true
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

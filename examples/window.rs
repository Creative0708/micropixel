#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use micropixel::*;

fn main() {
    let mut engine = EngineBuilder::default().dimensions(32, 32).build();

    engine.run(|ctx: &mut Context, _audio, pixels: &mut [[u8; 3]]| {
        let (width, height) = ctx.dimensions();
        for x in 0..width {
            for y in 0..height {
                pixels[(x + y * width) as usize] = [
                    [255, 0, 0],
                    [255, 255, 0],
                    [0, 255, 0],
                    [0, 255, 255],
                    [0, 0, 255],
                    [255, 0, 255],
                ][(x + y + ctx.current_frame() as u32 / 16) as usize / 2 % 6];
            }
        }
    });
}

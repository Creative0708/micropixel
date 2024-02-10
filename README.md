# Micropixel

**⚠️ Micropixel is still mostly under development. Expect bugs and some incomplete features. ⚠️**

Micropixel is a minimalist game framework written in Rust. It only handles window creation, input handling, texture rendering, and sound; everything else is controlled by your code.

## Example
```rs
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

```

[↗ examples/window.rs](examples/window.rs)
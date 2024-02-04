use std::num::NonZeroU32;

use glow::{Buffer, HasContext, NativeProgram, NativeTexture, Shader, VertexArray};
use glutin::{
    config::Config,
    context::{ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, Surface, SwapInterval, WindowSurface},
};
use glutin_winit::GlWindow;
use raw_window_handle::HasRawWindowHandle;
use winit::window::Window;

pub struct Gl {
    width: u32,
    height: u32,

    surface: Surface<WindowSurface>,

    bounding_box: (f32, f32, f32, f32),

    gl: glow::Context,
    gl_context: PossiblyCurrentContext,
    program: NativeProgram,
    vao: VertexArray,
    pos_vbo: Buffer,
    uv_vbo: Buffer,
    texture: NativeTexture,
}

impl Gl {
    pub fn new(window: &Window, gl_config: &Config, width: u32, height: u32) -> Self {
        let raw_window_handle = Some(window.raw_window_handle());

        let (not_current_context, shader_header) = [
            (
                ContextAttributesBuilder::new().build(raw_window_handle),
                // "#version 330",
                "",
            ),
            // (
            //     ContextAttributesBuilder::new()
            //         .with_context_api(ContextApi::Gles(None))
            //         .build(raw_window_handle),
            //     "#version 300 es\nprecision mediump float;",
            // ),
            // (
            //     ContextAttributesBuilder::new()
            //         .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1))))
            //         .build(raw_window_handle),
            //     "#version 120\n#define in attribute\n#define out varying",
            // ),
        ]
        .iter()
        .find_map(|(attr, shader_header)| unsafe {
            gl_config
                .display()
                .create_context(&gl_config, attr)
                .ok()
                .map(|ctx| (ctx, *shader_header))
        })
        .expect("no working contexts");

        unsafe {
            let display = gl_config.display();

            let surface_attributes = window.build_surface_attributes(Default::default());
            let surface = display
                .create_window_surface(&gl_config, &surface_attributes)
                .unwrap();

            let gl_context = not_current_context.make_current(&surface).unwrap();

            let gl = glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s));

            if let Err(res) = surface
                .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
            {
                eprintln!("Error setting vsync: {res:?}");
            }

            let program = gl.create_program().unwrap();

            let vertex_shader = Self::compile_shader(
                &gl,
                &program,
                &format!("{}\n{}", shader_header, include_str!("shader/vert.glsl")),
                glow::VERTEX_SHADER,
            )
            .unwrap();
            let fragment_shader = Self::compile_shader(
                &gl,
                &program,
                &format!("{}\n{}", shader_header, include_str!("shader/frag.glsl")),
                glow::FRAGMENT_SHADER,
            )
            .unwrap();

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            Self::delete_shader(&gl, &program, vertex_shader);
            Self::delete_shader(&gl, &program, fragment_shader);

            gl.use_program(Some(program));

            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let pos_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(pos_vbo));

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);

            let uv_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(uv_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::must_cast_slice::<f32, _>(&[0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0]),
                glow::STATIC_DRAW,
            );
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 0, 0);

            let texture = gl.create_texture().unwrap();
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_BORDER as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_BORDER as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB as i32,
                width as i32,
                height as i32,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                None,
            );

            gl.clear_color(0.0, 0.0, 0.0, 1.0);

            let obj = Self {
                width,
                height,

                bounding_box: (0.0, 0.0, 0.0, 0.0),
                surface,

                gl,
                gl_context,
                program,
                vao,
                pos_vbo,
                uv_vbo,
                texture,
            };

            obj.check_for_gl_error();

            obj
        }
    }

    #[inline]
    unsafe fn check_for_gl_error(&self) {
        if !cfg!(debug_assertions) {
            return;
        }
        let err = self.gl.get_error();
        if err != glow::NO_ERROR {
            panic!("opengl error {err:#02x}");
        }
    }

    unsafe fn compile_shader(
        gl: &glow::Context,
        program: &NativeProgram,
        source: &str,
        shader_type: u32,
    ) -> Result<Shader, String> {
        let shader = gl.create_shader(shader_type)?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            return Err(gl.get_shader_info_log(shader));
        }
        gl.attach_shader(*program, shader);
        Ok(shader)
    }

    unsafe fn delete_shader(gl: &glow::Context, program: &NativeProgram, shader: Shader) {
        gl.detach_shader(*program, shader);
        gl.delete_shader(shader);
    }

    pub fn draw(&mut self, _window: &mut Window, pixels: &[u8]) {
        debug_assert_eq!(pixels.len(), (self.width * self.height) as usize * 3);

        let gl = &self.gl;

        unsafe {
            gl.clear(glow::COLOR_BUFFER_BIT);

            // gl.active_texture(glow::TEXTURE0);
            // gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB as i32,
                self.width as i32,
                self.height as i32,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                Some(pixels),
            );
            // gl.bind_vertex_array(Some(self.vao));

            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);

            // self.check_for_gl_error();
        }
        // window.request_redraw();
        self.surface.swap_buffers(&self.gl_context).unwrap();
    }

    pub fn recalculate_dimensions(
        &mut self,
        window_width: u32,
        window_height: u32,
        fullscreen_data: Option<(u32, u32)>,
    ) {
        self.surface.resize(
            &self.gl_context,
            NonZeroU32::new(window_width).unwrap(),
            NonZeroU32::new(window_height).unwrap(),
        );

        let (window_width, window_height) = (window_width as f32, window_height as f32);

        let bounding_box = if let Some((target_width, target_height)) = fullscreen_data {
            let target_pixel_size = f32::min(
                window_width / target_width as f32,
                window_height / target_height as f32,
            );
            let min_pixel_size = f32::max(
                window_width / target_width as f32,
                window_height / target_height as f32,
            ) * 0.5;
            let pixel_size = f32::max(target_pixel_size, min_pixel_size);

            self.width = (window_width / pixel_size).ceil() as u32;
            self.height = (window_height / pixel_size).ceil() as u32;
            let radii = (
                self.width as f32 * pixel_size / window_width,
                self.height as f32 * pixel_size / window_height,
            );
            (-radii.0, -radii.1, radii.0, radii.1)
        } else {
            let window_radii = crate::calculate_fit_radii(
                self.width as f32,
                self.height as f32,
                window_width,
                window_height,
                0.1,
            );
            let radii = (
                window_radii.0 / window_width,
                window_radii.1 / window_height,
            );
            (-radii.0, -radii.1, radii.0, radii.1)
        };
        self.bounding_box = bounding_box;

        let gl = &mut self.gl;
        unsafe {
            gl.viewport(0, 0, window_width as i32, window_height as i32);

            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.pos_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::must_cast_slice(&[
                    bounding_box.0,
                    bounding_box.1,
                    bounding_box.2,
                    bounding_box.1,
                    bounding_box.0,
                    bounding_box.3,
                    bounding_box.2,
                    bounding_box.3,
                ]),
                glow::STATIC_DRAW,
            );
        }
    }

    #[inline]
    pub fn bounding_box(&self) -> (f32, f32, f32, f32) {
        self.bounding_box
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

impl Drop for Gl {
    fn drop(&mut self) {
        let gl = &self.gl;
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vao);
            gl.delete_buffer(self.pos_vbo);
            gl.delete_buffer(self.uv_vbo);
            gl.delete_texture(self.texture);
        }
    }
}

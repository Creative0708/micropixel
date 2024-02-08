pub struct Gl {
    width: u32,
    height: u32,

    bounding_box: (f32, f32, f32, f32),

    program: u32,
    vao: u32,
    pos_vbo: u32,
    uv_vbo: u32,
    texture: u32,
}

macro_rules! gl_load {
    ($($func:ident)+, $loader_function:ident) => {
        $(
            gl::$func::load_with(&mut $loader_function);
        )+
    };
}

impl Gl {
    pub fn new<F>(width: u32, height: u32, mut loader_function: F) -> Self
    where
        F: FnMut(&'static str) -> *const std::ffi::c_void,
    {
        unsafe {
            gl_load!(
                GetString CreateProgram CreateShader ShaderSource CompileShader AttachShader LinkProgram DetachShader DeleteShader UseProgram GenVertexArrays BindVertexArray GenBuffers BindBuffer EnableVertexAttribArray VertexAttribPointer GenTextures ActiveTexture BindTexture TexParameteri PixelStorei TexImage2D ClearColor Clear DrawArrays Viewport BufferData DeleteProgram DeleteVertexArrays DeleteBuffers DeleteTextures, loader_function);
            #[cfg(debug_assertions)]
            gl_load!(GetProgramiv GetShaderiv GetError, loader_function);

            // let version = std::ffi::CStr::from_ptr(gl::GetString(gl::VERSION) as *const _)
            //     .to_str()
            //     .unwrap();

            let program = gl::CreateProgram();

            unsafe fn compile_shader(program: u32, source: &str, shader_type: u32) -> u32 {
                let shader = gl::CreateShader(shader_type);
                gl::ShaderSource(
                    shader,
                    1,
                    &(source.as_bytes().as_ptr() as *const _),
                    &(source.len() as i32),
                );
                gl::CompileShader(shader);

                #[cfg(debug_assertions)]
                {
                    let mut status = 0;
                    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

                    if status != 1 {
                        panic!("shader compilation error");
                    }
                }

                gl::AttachShader(program, shader);

                shader
            }

            let vertex_shader =
                compile_shader(program, include_str!("shader/vert.glsl"), gl::VERTEX_SHADER);
            let fragment_shader = compile_shader(
                program,
                include_str!("shader/frag.glsl"),
                gl::FRAGMENT_SHADER,
            );

            gl::LinkProgram(program);

            #[cfg(debug_assertions)]
            {
                let mut status = 0;
                gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
                if status != 1 {
                    panic!("program error");
                }
            }

            unsafe fn delete_shader(program: u32, shader: u32) {
                gl::DetachShader(program, shader);
                gl::DeleteShader(shader);
            }

            delete_shader(program, vertex_shader);
            delete_shader(program, fragment_shader);

            gl::UseProgram(program);

            let mut vao = 0;
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            let mut vbo_buf = [0, 0];
            gl::GenBuffers(2, vbo_buf.as_mut_ptr());
            let [pos_vbo, uv_vbo] = vbo_buf;

            gl::BindBuffer(gl::ARRAY_BUFFER, pos_vbo);
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, 0, 0, std::ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, uv_vbo);
            let uv_data =
                bytemuck::must_cast_slice::<f32, u8>(&[0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                uv_data.len() as isize,
                uv_data.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, 0, 0, std::ptr::null());

            let mut texture = 0;
            gl::GenTextures(1, &mut texture);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_BORDER as i32,
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_BORDER as i32,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGB as i32,
                width as i32,
                height as i32,
                0,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );

            gl::ClearColor(0.0, 0.0, 0.0, 1.0);

            let obj = Self {
                width,
                height,

                bounding_box: (0.0, 0.0, 0.0, 0.0),

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
        #[cfg(debug_assertions)]
        {
            let err = gl::GetError();
            if err != gl::NO_ERROR {
                panic!("opengl error {err:#02x}");
            }
        }
    }

    pub fn draw(&mut self, pixels: &[u8]) {
        debug_assert_eq!(pixels.len(), (self.width * self.height) as usize * 3);

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGB as i32,
                self.width as i32,
                self.height as i32,
                0,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                pixels.as_ptr() as *const _,
            );

            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            self.check_for_gl_error();
        }
    }

    pub fn recalculate_dimensions_and_bounding_box(
        &mut self,
        window_width: u32,
        window_height: u32,
        fullscreen_target_dimensions: Option<(u32, u32)>,
    ) {
        ((self.width, self.height), self.bounding_box) =
            if let Some((target_width, target_height)) = fullscreen_target_dimensions {
                crate::platform::calculate_dimensions_and_bounding_box(
                    target_width,
                    target_height,
                    window_width,
                    window_height,
                    true,
                )
            } else {
                crate::platform::calculate_dimensions_and_bounding_box(
                    self.width,
                    self.height,
                    window_width,
                    window_height,
                    false,
                )
            };

        unsafe {
            gl::Viewport(0, 0, window_width as i32, window_height as i32);

            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.pos_vbo);
            let bounding_box = self.bounding_box;
            let pos_data_f32 = [
                bounding_box.0,
                bounding_box.1,
                bounding_box.2,
                bounding_box.1,
                bounding_box.0,
                bounding_box.3,
                bounding_box.2,
                bounding_box.3,
            ];
            let pos_data = bytemuck::must_cast_slice::<f32, u8>(&pos_data_f32);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                pos_data.len() as isize,
                pos_data.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
        }
    }
    pub fn current_bounding_box(&self) -> (f32, f32, f32, f32) {
        self.bounding_box
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn deinit(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
            gl::DeleteVertexArrays(1, &self.vao);
            let buffers = [self.pos_vbo, self.uv_vbo];
            gl::DeleteBuffers(2, buffers.as_ptr());
            gl::DeleteTextures(1, &self.texture);
        }
    }
}

use anyhow::{Context as _, Result, anyhow};
use glam::{Mat4, Vec3};
use glow::HasContext;

/// A linked vertex + fragment shader program.
pub struct ShaderProgram {
    program: glow::Program,
}

impl ShaderProgram {
    pub fn new(gl: &glow::Context, vertex_src: &str, fragment_src: &str) -> Result<Self> {
        unsafe {
            let vs = compile(gl, glow::VERTEX_SHADER, vertex_src).context("vertex shader")?;
            let fs = compile(gl, glow::FRAGMENT_SHADER, fragment_src).context("fragment shader")?;

            let program = gl.create_program().map_err(|e| anyhow!(e))?;
            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);
            gl.detach_shader(program, vs);
            gl.detach_shader(program, fs);
            gl.delete_shader(vs);
            gl.delete_shader(fs);

            if !gl.get_program_link_status(program) {
                let log = gl.get_program_info_log(program);
                gl.delete_program(program);
                return Err(anyhow!("shader link failed: {log}"));
            }
            Ok(Self { program })
        }
    }

    pub fn bind(&self, gl: &glow::Context) {
        unsafe { gl.use_program(Some(self.program)) }
    }

    pub fn set_mat4(&self, gl: &glow::Context, name: &str, value: &Mat4) {
        unsafe {
            let loc = gl.get_uniform_location(self.program, name);
            gl.uniform_matrix_4_f32_slice(loc.as_ref(), false, &value.to_cols_array());
        }
    }

    pub fn set_vec3(&self, gl: &glow::Context, name: &str, value: Vec3) {
        unsafe {
            let loc = gl.get_uniform_location(self.program, name);
            gl.uniform_3_f32(loc.as_ref(), value.x, value.y, value.z);
        }
    }

    pub fn set_f32(&self, gl: &glow::Context, name: &str, value: f32) {
        unsafe {
            let loc = gl.get_uniform_location(self.program, name);
            gl.uniform_1_f32(loc.as_ref(), value);
        }
    }

    pub fn set_i32(&self, gl: &glow::Context, name: &str, value: i32) {
        unsafe {
            let loc = gl.get_uniform_location(self.program, name);
            gl.uniform_1_i32(loc.as_ref(), value);
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe { gl.delete_program(self.program) }
    }
}

unsafe fn compile(gl: &glow::Context, kind: u32, src: &str) -> Result<glow::Shader> {
    unsafe {
        let shader = gl.create_shader(kind).map_err(|e| anyhow!(e))?;
        gl.shader_source(shader, src);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            return Err(anyhow!("shader compile failed: {log}"));
        }
        Ok(shader)
    }
}

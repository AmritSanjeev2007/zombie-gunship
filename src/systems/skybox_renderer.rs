use std::path::PathBuf;

use anyhow::{Result, anyhow};
use glam::Mat4;
use glow::HasContext;

use crate::components::Skybox;
use crate::engine::{FrameContext, System};
use crate::render::{ShaderProgram, Texture, texture::Wrap};

const VERTEX_PATH: &'static str = "./shaders/skybox.vert";
const FRAGMENT_PATH: &'static str = "./shaders/skybox.frag";

struct Gpu {
    shader: ShaderProgram,
    vao: glow::VertexArray,
    texture: Option<(PathBuf, Texture)>,
}

/// Draws the first [`Skybox`] in the world behind everything else.
/// Register it right after clearing and before opaque geometry.
#[derive(Default)]
pub struct SkyboxRendererSystem {
    gpu: Option<Gpu>,
    failed: bool,
}

impl SkyboxRendererSystem {
    fn init(gl: &glow::Context) -> Result<Gpu> {
        let vertex_src = std::fs::read_to_string(VERTEX_PATH)?;
        let fragment_src = std::fs::read_to_string(FRAGMENT_PATH)?;

        let shader = ShaderProgram::new(gl, &vertex_src, &fragment_src)?;
        let vao = unsafe { gl.create_vertex_array().map_err(|e| anyhow!(e))? };
        Ok(Gpu {
            shader,
            vao,
            texture: None,
        })
    }
}

impl System for SkyboxRendererSystem {
    fn name(&self) -> &'static str {
        "Skybox renderer"
    }

    fn render(&mut self, ctx: &mut FrameContext) {
        let gl = ctx.gl;
        let Some(camera) = ctx.resources.camera else {
            return;
        };
        let mut query = ctx.world.query::<&Skybox>();
        let Some(skybox) = query.iter().next() else {
            return;
        };

        if self.failed {
            return;
        }
        if self.gpu.is_none() {
            match Self::init(gl) {
                Ok(gpu) => self.gpu = Some(gpu),
                Err(err) => {
                    eprintln!("skybox: {err:#}");
                    self.failed = true;
                    return;
                }
            }
        }
        let gpu = self.gpu.as_mut().unwrap();

        // (Re)load the texture if the component points at a different file.
        if gpu.texture.as_ref().map(|(p, _)| p) != Some(&skybox.texture) {
            if let Some((_, old)) = gpu.texture.take() {
                old.destroy(gl);
            }
            match Texture::from_file(gl, &skybox.texture, Wrap::Repeat, true, true) {
                Ok(tex) => gpu.texture = Some((skybox.texture.clone(), tex)),
                Err(err) => {
                    eprintln!("skybox: {err:#}");
                    self.failed = true;
                    return;
                }
            }
        }
        let (_, texture) = gpu.texture.as_ref().unwrap();

        // Drop the translation so the sky stays infinitely far away.
        let rotation_only = Mat4::from_mat3(glam::Mat3::from_mat4(camera.view));
        let inv_view_proj = (camera.projection * rotation_only).inverse();

        gpu.shader.bind(gl);
        gpu.shader.set_mat4(gl, "u_inv_view_proj", &inv_view_proj);
        gpu.shader.set_vec3(gl, "u_tint", skybox.tint);
        gpu.shader.set_f32(gl, "u_rotation", skybox.rotation);
        gpu.shader.set_i32(gl, "u_panorama", 0);
        let fog = &ctx.resources.fog;
        gpu.shader.set_vec3(gl, "u_fog_color", fog.color);
        gpu.shader.set_f32(gl, "u_fog_blend", fog.sky_blend.sin());
        texture.bind(gl, 0);

        unsafe {
            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            gl.disable(glow::CULL_FACE);
            gl.bind_vertex_array(Some(gpu.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.bind_vertex_array(None);
            gl.depth_mask(true);
        }
    }

    fn destroy(&mut self, gl: &glow::Context) {
        if let Some(gpu) = self.gpu.take() {
            gpu.shader.destroy(gl);
            if let Some((_, tex)) = gpu.texture {
                tex.destroy(gl);
            }
            unsafe { gl.delete_vertex_array(gpu.vao) };
        }
    }
}

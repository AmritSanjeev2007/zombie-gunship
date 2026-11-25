use std::collections::HashMap;

use glam::Vec3;
use glow::HasContext;

use crate::components::{MeshRenderer, Primitive, Transform};
use crate::engine::{FrameContext, System};
use crate::render::{GpuMesh, ShaderProgram, fog, mesh};

const VERTEX_PATH: &'static str = "./shaders/mesh.vert";
const FRAGMENT_PATH: &'static str = "./shaders/mesh.frag";

/// Draws every entity that has a [`MeshRenderer`] and a [`Transform`].
pub struct MeshRendererSystem {
    shader: Option<ShaderProgram>,
    meshes: HashMap<Primitive, GpuMesh>,
    /// Direction towards the sun.
    pub sun_direction: Vec3,
}

impl Default for MeshRendererSystem {
    fn default() -> Self {
        Self {
            shader: None,
            meshes: HashMap::new(),
            sun_direction: Vec3::new(0.4, 1.0, 0.3).normalize(),
        }
    }
}

impl MeshRendererSystem {
    fn mesh(&mut self, gl: &glow::Context, primitive: Primitive) -> Option<&GpuMesh> {
        if let std::collections::hash_map::Entry::Vacant(e) = self.meshes.entry(primitive) {
            let built = match primitive {
                Primitive::Cube => GpuMesh::new(gl, &mesh::cube_vertices(), glow::TRIANGLES),
                Primitive::Grid => GpuMesh::new(gl, &mesh::grid_vertices(40), glow::LINES),
            };
            match built {
                Ok(m) => {
                    e.insert(m);
                }
                Err(err) => {
                    eprintln!("mesh renderer: {err:#}");
                    return None;
                }
            }
        }
        self.meshes.get(&primitive)
    }
}

impl System for MeshRendererSystem {
    fn name(&self) -> &'static str {
        "Mesh renderer"
    }

    fn render(&mut self, ctx: &mut FrameContext) {
        let gl = ctx.gl;
        let Some(camera) = ctx.resources.camera else {
            return;
        };

        if self.shader.is_none() {
            let vertex_shader_load_result = std::fs::read_to_string(VERTEX_PATH);
            let fragment_shader_load_result = std::fs::read_to_string(FRAGMENT_PATH);

            if let Err(err) = vertex_shader_load_result.as_ref().and(fragment_shader_load_result.as_ref()) {
                eprintln!("mesh renderer: {err:#}");
                return;
            }

            let vertex_src = vertex_shader_load_result.unwrap();
            let fragment_src = fragment_shader_load_result.unwrap();

            match ShaderProgram::new(gl, &vertex_src, &fragment_src) {
                Ok(s) => self.shader = Some(s),
                Err(err) => {
                    eprintln!("mesh renderer: {err:#}");
                    return;
                }
            }
        }

        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
        }

        let view_proj = camera.view_projection();
        let sun = self.sun_direction.normalize();
        let items: Vec<(MeshRenderer, Transform)> = ctx
            .world
            .query::<(&MeshRenderer, &Transform)>()
            .iter()
            .map(|(m, t)| (*m, *t))
            .collect();

        for (renderer, transform) in items {
            if self.mesh(gl, renderer.primitive).is_none() {
                continue;
            }
            let shader = self.shader.as_ref().unwrap();
            shader.bind(gl);
            shader.set_mat4(gl, "u_view_proj", &view_proj);
            shader.set_vec3(gl, "u_sun_dir", sun);
            shader.set_mat4(gl, "u_model", &transform.matrix());
            shader.set_vec3(gl, "u_color", renderer.color);
            shader.set_f32(gl, "u_lit", if renderer.lit { 1.0 } else { 0.0 });
            fog::set_uniforms(gl, shader, &ctx.resources.fog, ctx.resources.map.as_ref());
            self.meshes[&renderer.primitive].draw(gl);
        }

        unsafe { gl.disable(glow::CULL_FACE) };
    }

    fn destroy(&mut self, gl: &glow::Context) {
        if let Some(shader) = self.shader.take() {
            shader.destroy(gl);
        }
        for (_, mesh) in self.meshes.drain() {
            mesh.destroy(gl);
        }
    }
}

use glam::{Vec2, Vec3};
use glow::HasContext;

use crate::components::Terrain;
use crate::engine::{FrameContext, System};
use crate::map::Map;
use crate::render::{GpuMesh, ShaderProgram, mesh};

const TERRAIN_VERTEX: &str = "./shaders/terrain.vert";
const TERRAIN_FRAGMENT: &str = "./shaders/terrain.frag";
const LINE_VERTEX: &str = "./shaders/mesh.vert";
const LINE_FRAGMENT: &str = "./shaders/mesh.frag";

/// Lift overlay lines this far above the ground so they don't z-fight.
const OVERLAY_LIFT: f32 = 0.2;

/// Meshes built from one map revision.
struct MapMeshes {
    revision: u64,
    terrain: GpuMesh,
    routes: GpuMesh,
    links: GpuMesh,
    ring: GpuMesh,
}

impl MapMeshes {
    fn build(gl: &glow::Context, map: &Map, revision: u64) -> anyhow::Result<Self> {
        let (vertices, indices) = mesh::terrain_mesh(&map.heights);
        let terrain = GpuMesh::new_indexed(gl, &vertices, &indices, glow::TRIANGLES)?;

        let lift = |p: Vec2| map.surface(p) + Vec3::Y * OVERLAY_LIFT;
        let routes: Vec<Vec<Vec3>> = map
            .routes()
            .map(|r| r.points.iter().map(|&p| lift(p)).collect())
            .collect();
        // Straight links between tree nodes show the tree itself.
        let links: Vec<Vec<Vec3>> = map
            .paths
            .iter()
            .flat_map(|tree| {
                tree.nodes.iter().filter_map(|n| {
                    let parent = tree.nodes[n.parent?].pos;
                    Some(vec![lift(n.pos) + Vec3::Y, lift(parent) + Vec3::Y])
                })
            })
            .collect();
        let ring: Vec<Vec3> = map.ring_road.iter().map(|&p| lift(p)).collect();

        let lines = |ls: &[Vec<Vec3>]| mesh::polyline_vertices(ls.iter().map(Vec::as_slice));
        Ok(Self {
            revision,
            terrain,
            routes: GpuMesh::new(gl, &lines(&routes), glow::LINES)?,
            links: GpuMesh::new(gl, &lines(&links), glow::LINES)?,
            ring: GpuMesh::new(gl, &mesh::polyline_vertices([ring.as_slice()]), glow::LINES)?,
        })
    }

    fn destroy(&self, gl: &glow::Context) {
        for m in [&self.terrain, &self.routes, &self.links, &self.ring] {
            m.destroy(gl);
        }
    }
}

fn load_shader(gl: &glow::Context, vertex: &str, fragment: &str) -> anyhow::Result<ShaderProgram> {
    let vertex_src = std::fs::read_to_string(vertex)?;
    let fragment_src = std::fs::read_to_string(fragment)?;
    ShaderProgram::new(gl, &vertex_src, &fragment_src)
}

/// Draws the map's heightfield for every [`Terrain`] entity, plus an
/// optional overlay of the zombie paths. Rebuilds its meshes whenever
/// [`Resources::map_revision`](crate::engine::Resources::map_revision) changes.
pub struct TerrainRendererSystem {
    terrain_shader: Option<ShaderProgram>,
    line_shader: Option<ShaderProgram>,
    meshes: Option<MapMeshes>,
    failed: bool,
    pub sun_direction: Vec3,
}

impl Default for TerrainRendererSystem {
    fn default() -> Self {
        Self {
            terrain_shader: None,
            line_shader: None,
            meshes: None,
            failed: false,
            sun_direction: Vec3::new(0.4, 1.0, 0.3).normalize(),
        }
    }
}

impl TerrainRendererSystem {
    fn ensure_shaders(&mut self, gl: &glow::Context) -> bool {
        if self.failed {
            return false;
        }
        if self.terrain_shader.is_none() {
            match load_shader(gl, TERRAIN_VERTEX, TERRAIN_FRAGMENT)
                .and_then(|t| Ok((t, load_shader(gl, LINE_VERTEX, LINE_FRAGMENT)?)))
            {
                Ok((terrain, line)) => {
                    self.terrain_shader = Some(terrain);
                    self.line_shader = Some(line);
                }
                Err(err) => {
                    eprintln!("terrain renderer: {err:#}");
                    self.failed = true;
                    return false;
                }
            }
        }
        true
    }
}

impl System for TerrainRendererSystem {
    fn name(&self) -> &'static str {
        "Terrain renderer"
    }

    fn render(&mut self, ctx: &mut FrameContext) {
        let gl = ctx.gl;
        let (Some(camera), Some(map)) = (ctx.resources.camera, ctx.resources.map.as_ref()) else {
            return;
        };
        let Some(terrain) = ctx.world.query::<&Terrain>().iter().next().copied() else {
            return;
        };
        if !self.ensure_shaders(gl) {
            return;
        }

        let revision = ctx.resources.map_revision;
        if self.meshes.as_ref().is_none_or(|m| m.revision != revision) {
            if let Some(old) = self.meshes.take() {
                old.destroy(gl);
            }
            match MapMeshes::build(gl, map, revision) {
                Ok(m) => self.meshes = Some(m),
                Err(err) => {
                    eprintln!("terrain renderer: {err:#}");
                    return;
                }
            }
        }
        let meshes = self.meshes.as_ref().unwrap();

        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
        }

        let view_proj = camera.view_projection();
        let shader = self.terrain_shader.as_ref().unwrap();
        shader.bind(gl);
        shader.set_mat4(gl, "u_view_proj", &view_proj);
        shader.set_vec3(gl, "u_sun_dir", self.sun_direction.normalize());
        shader.set_f32(gl, "u_max_height", map.config.max_height);
        shader.set_vec3(gl, "u_low_color", terrain.low_color);
        shader.set_vec3(gl, "u_high_color", terrain.high_color);
        shader.set_vec3(gl, "u_rock_color", terrain.rock_color);
        meshes.terrain.draw(gl);

        unsafe { gl.disable(glow::CULL_FACE) };

        if terrain.show_paths {
            let shader = self.line_shader.as_ref().unwrap();
            shader.bind(gl);
            shader.set_mat4(gl, "u_view_proj", &view_proj);
            shader.set_mat4(gl, "u_model", &glam::Mat4::IDENTITY);
            shader.set_vec3(gl, "u_sun_dir", self.sun_direction);
            shader.set_f32(gl, "u_lit", 0.0);
            for (mesh, color) in [
                (&meshes.routes, Vec3::new(1.0, 0.15, 0.1)),
                (&meshes.links, Vec3::new(0.2, 0.9, 1.0)),
                (&meshes.ring, Vec3::new(1.0, 0.85, 0.2)),
            ] {
                shader.set_vec3(gl, "u_color", color);
                mesh.draw(gl);
            }
        }
    }

    fn destroy(&mut self, gl: &glow::Context) {
        for shader in [self.terrain_shader.take(), self.line_shader.take()]
            .into_iter()
            .flatten()
        {
            shader.destroy(gl);
        }
        if let Some(m) = self.meshes.take() {
            m.destroy(gl);
        }
    }
}

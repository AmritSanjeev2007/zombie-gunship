//! Fog uniforms shared by every shader that draws world geometry
//! (`terrain.frag`, `mesh.frag`), which each carry the same `apply_fog`.

use glam::Vec2;

use super::ShaderProgram;
use crate::engine::Fog;
use crate::map::Map;

pub fn set_uniforms(gl: &glow::Context, shader: &ShaderProgram, fog: &Fog, map: Option<&Map>) {
    // With no map, push the "edge" far away so nothing is fogged.
    let (half, plain) = map.map_or((Vec2::splat(1e6), 0.0), |m| {
        (m.config.size * 0.5, m.apron.plain_height)
    });
    let end = fog.end.max(fog.start + 0.01);
    let density = if fog.ground {
        fog.ground_density.clamp(0.0, 1.0)
    } else {
        0.0
    };
    shader.set_vec3(gl, "u_fog_color", fog.color);
    shader.set_vec3(
        gl,
        "u_fog_range",
        glam::Vec3::new(fog.start, end, fog.outside_dim),
    );
    shader.set_vec3(
        gl,
        "u_fog_ground",
        glam::Vec3::new(density, plain, fog.ground_thickness.max(0.01)),
    );
    shader.set_vec3(gl, "u_map_half", half.extend(0.0));
}

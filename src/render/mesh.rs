use anyhow::{Result, anyhow};
use glam::Vec3;
use glow::HasContext;

use crate::map::{Apron, Heightfield};

/// Interleaved vertex: position then normal.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

/// Vertex data uploaded to the GPU, ready to draw.
pub struct GpuMesh {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    /// Index buffer; `None` draws the vertices in order.
    ebo: Option<glow::Buffer>,
    /// Vertices drawn: the index count for indexed meshes.
    vertex_count: i32,
    /// `glow::TRIANGLES`, `glow::LINES`, ...
    mode: u32,
}

impl GpuMesh {
    pub fn new(gl: &glow::Context, vertices: &[Vertex], mode: u32) -> Result<Self> {
        Self::upload(gl, vertices, None, mode)
    }

    /// Like [`GpuMesh::new`], but draws `indices` into `vertices`. Big grids
    /// such as terrain share each vertex between up to six triangles.
    pub fn new_indexed(
        gl: &glow::Context,
        vertices: &[Vertex],
        indices: &[u32],
        mode: u32,
    ) -> Result<Self> {
        Self::upload(gl, vertices, Some(indices), mode)
    }

    fn upload(
        gl: &glow::Context,
        vertices: &[Vertex],
        indices: Option<&[u32]>,
        mode: u32,
    ) -> Result<Self> {
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(vertices.as_ptr().cast(), std::mem::size_of_val(vertices))
        };
        let stride = std::mem::size_of::<Vertex>() as i32;

        unsafe {
            let vao = gl.create_vertex_array().map_err(|e| anyhow!(e))?;
            let vbo = gl.create_buffer().map_err(|e| anyhow!(e))?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 12);

            // The element buffer binding is stored in the VAO, so bind it
            // while the VAO is still bound.
            let ebo = match indices {
                Some(indices) => {
                    let ebo = gl.create_buffer().map_err(|e| anyhow!(e))?;
                    let bytes: &[u8] = std::slice::from_raw_parts(
                        indices.as_ptr().cast(),
                        std::mem::size_of_val(indices),
                    );
                    gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
                    gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
                    Some(ebo)
                }
                None => None,
            };

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);

            Ok(Self {
                vao,
                vbo,
                ebo,
                vertex_count: indices.map_or(vertices.len(), <[u32]>::len) as i32,
                mode,
            })
        }
    }

    pub fn draw(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_vertex_array(Some(self.vao));
            if self.ebo.is_some() {
                gl.draw_elements(self.mode, self.vertex_count, glow::UNSIGNED_INT, 0);
            } else {
                gl.draw_arrays(self.mode, 0, self.vertex_count);
            }
            gl.bind_vertex_array(None);
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_vertex_array(self.vao);
            gl.delete_buffer(self.vbo);
            if let Some(ebo) = self.ebo {
                gl.delete_buffer(ebo);
            }
        }
    }
}

/// Unit cube centred on the origin, 36 vertices with flat normals.
pub fn cube_vertices() -> Vec<Vertex> {
    let faces: [(Vec3, Vec3, Vec3); 6] = [
        // (normal, u axis, v axis) with u × v = normal
        (Vec3::X, Vec3::NEG_Z, Vec3::Y),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::X, Vec3::NEG_Z),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
    ];
    let mut out = Vec::with_capacity(36);
    for (n, u, v) in faces {
        let c = n * 0.5;
        let corners = [
            c - u * 0.5 - v * 0.5,
            c + u * 0.5 - v * 0.5,
            c + u * 0.5 + v * 0.5,
            c - u * 0.5 + v * 0.5,
        ];
        for i in [0, 1, 2, 0, 2, 3] {
            out.push(Vertex {
                position: corners[i].to_array(),
                normal: n.to_array(),
            });
        }
    }
    out
}

/// Square of `divisions × divisions` cells on the XZ plane, 1 unit across,
/// as line pairs.
pub fn grid_vertices(divisions: u32) -> Vec<Vertex> {
    let mut out = Vec::new();
    let normal = Vec3::Y.to_array();
    for i in 0..=divisions {
        let t = i as f32 / divisions as f32 - 0.5;
        for (a, b) in [
            ([t, 0.0, -0.5], [t, 0.0, 0.5]),
            ([-0.5, 0.0, t], [0.5, 0.0, t]),
        ] {
            out.push(Vertex {
                position: a,
                normal,
            });
            out.push(Vertex {
                position: b,
                normal,
            });
        }
    }
    out
}

/// Triangle mesh of a heightfield: one vertex per sample with smooth
/// normals, two counter-clockwise (seen from above) triangles per cell.
pub fn terrain_mesh(field: &Heightfield) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(field.cols * field.rows);
    for j in 0..field.rows {
        for i in 0..field.cols {
            let p = field.position(i, j);
            vertices.push(Vertex {
                position: [p.x, field.get(i, j), p.y],
                normal: field.normal(i, j).to_array(),
            });
        }
    }
    let mut indices = Vec::with_capacity((field.cols - 1) * (field.rows - 1) * 6);
    for j in 0..field.rows - 1 {
        for i in 0..field.cols - 1 {
            let at = |di: usize, dj: usize| field.index(i + di, j + dj) as u32;
            indices.extend([at(0, 0), at(0, 1), at(1, 0), at(1, 0), at(0, 1), at(1, 1)]);
        }
    }
    (vertices, indices)
}

/// Triangle mesh of the land around the map: a strip of quads between each
/// pair of neighbouring rings, wrapping around the rectangle.
pub fn apron_mesh(apron: &Apron) -> (Vec<Vertex>, Vec<u32>) {
    let vertices: Vec<Vertex> = apron
        .positions
        .iter()
        .zip(&apron.normals)
        .map(|(p, n)| Vertex {
            position: p.to_array(),
            normal: n.to_array(),
        })
        .collect();
    let mut indices = Vec::with_capacity((apron.rings - 1) * apron.ring_len * 6);
    for k in 0..apron.rings - 1 {
        for i in 0..apron.ring_len {
            let a = apron.index(k, i) as u32;
            let b = apron.index(k, i + 1) as u32;
            let c = apron.index(k + 1, i) as u32;
            let d = apron.index(k + 1, i + 1) as u32;
            indices.extend([a, b, c, b, d, c]);
        }
    }
    (vertices, indices)
}

/// Line pairs following each polyline (`LINES` mode).
pub fn polyline_vertices<'a>(lines: impl IntoIterator<Item = &'a [Vec3]>) -> Vec<Vertex> {
    let normal = Vec3::Y.to_array();
    let mut out = Vec::new();
    for line in lines {
        for pair in line.windows(2) {
            for p in pair {
                out.push(Vertex {
                    position: p.to_array(),
                    normal,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apron_triangles_face_up() {
        let map = crate::map::generate(&crate::map::MapConfig {
            resolution: 0.5,
            ..Default::default()
        });
        let (verts, indices) = apron_mesh(&map.apron);
        for tri in indices.chunks(3) {
            let [a, b, c] =
                [tri[0], tri[1], tri[2]].map(|i| Vec3::from(verts[i as usize].position));
            // Corner fans start from a single point, so some are degenerate.
            let n = (b - a).cross(c - a);
            assert!(n.y > 0.0 || n.length() < 1e-4, "{n}");
        }
    }

    #[test]
    fn terrain_triangles_face_up() {
        let field = Heightfield::new(4, 3, glam::Vec2::new(3.0, 2.0));
        let (verts, indices) = terrain_mesh(&field);
        assert_eq!(verts.len(), 12);
        assert_eq!(indices.len(), 3 * 2 * 6);
        for tri in indices.chunks(3) {
            let [a, b, c] =
                [tri[0], tri[1], tri[2]].map(|i| Vec3::from(verts[i as usize].position));
            assert!((b - a).cross(c - a).y > 0.0);
        }
    }

    #[test]
    fn cube_faces_wind_counter_clockwise() {
        let verts = cube_vertices();
        assert_eq!(verts.len(), 36);
        for tri in verts.chunks(3) {
            let [a, b, c] = [tri[0], tri[1], tri[2]].map(|v| Vec3::from(v.position));
            let n = (b - a).cross(c - a).normalize();
            assert!(n.abs_diff_eq(Vec3::from(tri[0].normal), 1e-5));
        }
    }
}

//! The land around the map: rings of terrain growing outward from the map's
//! edge, continuing the same noise and flattening into a plain, so the
//! battlefield doesn't float in empty space.
//!
//! Ring 0 is the map's own edge (same positions, heights and normals), so
//! the seam is watertight. Ring spacing grows geometrically: fine near the
//! map, coarse far away where fog hides the detail anyway. Rings are
//! rounded at the corners, so a ring vertex's distance from the map is
//! exactly its ring's distance, matching how the fog measures it.

use glam::{Vec2, Vec3};

use super::pathing::smoothstep;
use super::terrain::{FLOOR_FRACTION, Heightfield, NoiseConfig, Perlin, base_height};

/// Shape of the land around the map. Distances are from the map's edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApronConfig {
    /// How far the noisy rings reach.
    pub extent: f32,
    /// One last flat ring this far out, so the ground runs to the horizon.
    pub far_extent: f32,
    /// Each ring is this much wider than the one before.
    pub growth: f32,
    /// Hills start flattening here...
    pub flatten_start: f32,
    /// ...and are a flat plain from here on.
    pub flatten_end: f32,
    /// Distance over which craters cut by the map's edge fade into the
    /// surrounding terrain.
    pub edge_blend: f32,
}

impl Default for ApronConfig {
    fn default() -> Self {
        Self {
            extent: 260.0,
            far_extent: 4000.0,
            growth: 1.15,
            flatten_start: 20.0,
            flatten_end: 160.0,
            edge_blend: 8.0,
        }
    }
}

/// Grid of `rings × ring_len` vertices; vertex `i` of every ring lies on the
/// same outward line from the map's edge.
#[derive(Clone, Debug, PartialEq)]
pub struct Apron {
    pub ring_len: usize,
    pub rings: usize,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    /// Height of the flat plain.
    pub plain_height: f32,
}

impl Apron {
    pub fn index(&self, ring: usize, i: usize) -> usize {
        ring * self.ring_len + i % self.ring_len
    }
}

/// Directions fanned around each corner, so rings round the corners and
/// every ring vertex sits exactly its ring's distance from the map.
const CORNER_STEPS: usize = 8;

/// Map edge samples in order around the rectangle, each with the unit
/// direction it moves outward. Each corner sample is repeated with
/// directions sweeping from one edge's normal to the next.
fn perimeter(field: &Heightfield) -> Vec<(usize, usize, Vec2)> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let (c, r) = (field.cols - 1, field.rows - 1);
    let mut out = Vec::with_capacity(2 * (c + r) + 4 * CORNER_STEPS);
    let corner = |out: &mut Vec<_>, i, j, from: f32| {
        for s in 0..=CORNER_STEPS {
            let angle = from + FRAC_PI_2 * s as f32 / CORNER_STEPS as f32;
            out.push((i, j, Vec2::from_angle(angle)));
        }
    };
    corner(&mut out, 0, 0, PI);
    for i in 1..c {
        out.push((i, 0, Vec2::NEG_Y));
    }
    corner(&mut out, c, 0, 1.5 * PI);
    for j in 1..r {
        out.push((c, j, Vec2::X));
    }
    corner(&mut out, c, r, 0.0);
    for i in (1..c).rev() {
        out.push((i, r, Vec2::Y));
    }
    corner(&mut out, 0, r, FRAC_PI_2);
    for j in (1..r).rev() {
        out.push((0, j, Vec2::NEG_X));
    }
    out
}

/// Distance of each ring from the map's edge.
fn ring_distances(first_step: f32, cfg: &ApronConfig) -> Vec<f32> {
    let mut d = vec![0.0];
    let mut step = first_step.max(0.05);
    while *d.last().unwrap() < cfg.extent && d.len() < 256 {
        d.push(d.last().unwrap() + step);
        step *= cfg.growth.max(1.0);
    }
    d.push(cfg.far_extent.max(cfg.extent * 2.0));
    d
}

pub fn build(
    field: &Heightfield,
    noise: &Perlin,
    noise_cfg: &NoiseConfig,
    max_height: f32,
    cfg: &ApronConfig,
) -> Apron {
    let edge = perimeter(field);
    let ring_len = edge.len();
    let spacing = field.spacing();
    let distances = ring_distances(spacing.x.min(spacing.y), cfg);
    let rings = distances.len();
    let plain = FLOOR_FRACTION * max_height;
    let last = rings - 1;

    let mut positions = Vec::with_capacity(rings * ring_len);
    for (k, &d) in distances.iter().enumerate() {
        for &(i, j, out) in &edge {
            let edge_p = field.position(i, j);
            let edge_h = field.get(i, j);
            let h = if k == 0 {
                edge_h
            } else if k == last {
                plain
            } else {
                let p = edge_p + out * d;
                // Whatever carving cut the edge fades out quickly...
                let carved = edge_h - base_height(noise, noise_cfg, max_height, edge_p);
                let hills = base_height(noise, noise_cfg, max_height, p)
                    + carved * (-d / cfg.edge_blend.max(0.01)).exp();
                // ...and the hills settle into the plain further out.
                let t = smoothstep(cfg.flatten_start, cfg.flatten_end, d);
                hills + (plain - hills) * t
            };
            let p = edge_p + out * d;
            positions.push(Vec3::new(p.x, h, p.y));
        }
    }

    let at = |k: usize, i: usize| positions[k * ring_len + i % ring_len];
    let mut normals = Vec::with_capacity(positions.len());
    for k in 0..rings {
        for (i, &(gi, gj, _)) in edge.iter().enumerate() {
            if k == 0 {
                normals.push(field.normal(gi, gj));
                continue;
            }
            let along = at(k, i + 1) - at(k, i + ring_len - 1);
            let across = at((k + 1).min(last), i) - at(k - 1, i);
            // Ring 1 corners touch at ring 0, so `along` can vanish there.
            let mut n = along.cross(across).normalize_or(Vec3::Y);
            if n.y < 0.0 {
                n = -n;
            }
            normals.push(n);
        }
    }

    Apron {
        ring_len,
        rings,
        positions,
        normals,
        plain_height: plain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{MapConfig, generate};

    #[test]
    fn apron_starts_on_the_map_edge_and_ends_flat() {
        let map = generate(&MapConfig {
            resolution: 0.6,
            ..Default::default()
        });
        let apron = &map.apron;
        let f = &map.heights;
        assert_eq!(
            apron.ring_len,
            2 * (f.cols - 2 + f.rows - 2) + 4 * (CORNER_STEPS + 1)
        );
        for (i, &(gi, gj, _)) in perimeter(f).iter().enumerate() {
            let p = apron.positions[apron.index(0, i)];
            let q = f.position(gi, gj);
            assert_eq!((p.x, p.y, p.z), (q.x, f.get(gi, gj), q.y));
        }
        for i in 0..apron.ring_len {
            let p = apron.positions[apron.index(apron.rings - 1, i)];
            assert_eq!(p.y, apron.plain_height);
            assert!(p.x.abs() > 1000.0 || p.z.abs() > 1000.0);
        }
        // Rounded corners: each ring is a constant distance from the map.
        let half = f.size * 0.5;
        for k in 1..apron.rings {
            let ds: Vec<f32> = (0..apron.ring_len)
                .map(|i| {
                    let p = apron.positions[apron.index(k, i)];
                    (Vec2::new(p.x, p.z).abs() - half).max(Vec2::ZERO).length()
                })
                .collect();
            let (lo, hi) = ds
                .iter()
                .fold((f32::MAX, 0.0f32), |(l, h), &d| (l.min(d), h.max(d)));
            assert!(hi - lo < 1e-2 * hi.max(1.0), "ring {k}: {lo}..{hi}");
        }
        assert!(apron.normals.iter().all(|n| n.y > 0.0));
    }
}

//! Digging craters and valleys into the base terrain.
//!
//! Every feature contributes a weight in `[0, 1]` from a flat-bottomed
//! profile. Overlapping features take the maximum, never the sum, so shared
//! valley trunks and crater/valley junctions aren't dug twice.

use glam::Vec2;

use super::pathing::{PathTree, smoothstep};
use super::terrain::{BaseTerrain, Heightfield, Perlin};
use super::{MapConfig, Objective, SpawnPoint};

/// Craters at objectives and spawns. Sizes in world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CraterConfig {
    pub objective_radius: f32,
    pub spawn_radius: f32,
    /// Share of the radius that is flat floor.
    pub floor_fraction: f32,
    pub depth: f32,
    /// Height of the raised lip around each crater (0 for none).
    pub rim_height: f32,
    pub rim_width: f32,
    /// How far the outline strays from a perfect circle, as a share of the
    /// radius.
    pub wobble: f32,
}

impl Default for CraterConfig {
    fn default() -> Self {
        Self {
            objective_radius: 11.0,
            spawn_radius: 7.0,
            floor_fraction: 0.45,
            depth: 4.0,
            rim_height: 1.2,
            rim_width: 2.5,
            wobble: 0.15,
        }
    }
}

/// Valleys along zombie routes. Sizes in world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValleyConfig {
    pub depth: f32,
    /// Full width where the route leaves its spawn.
    pub width_at_spawn: f32,
    /// Full width as it reaches the objective.
    pub width_at_objective: f32,
    /// Distance from the objective over which the valley narrows back to
    /// `width_at_spawn`.
    pub widen_distance: f32,
    /// Share of the width that is flat floor.
    pub floor_fraction: f32,
    /// How strongly floors blend toward the smooth terrain layer (0..1).
    pub floor_smoothing: f32,
}

impl Default for ValleyConfig {
    fn default() -> Self {
        Self {
            depth: 2.5,
            width_at_spawn: 9.0,
            width_at_objective: 14.0,
            widen_distance: 50.0,
            floor_fraction: 0.2,
            floor_smoothing: 0.85,
        }
    }
}

/// The road carved along the loop joining the objectives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingConfig {
    pub enabled: bool,
    pub width: f32,
    pub depth: f32,
}

impl Default for RingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            width: 9.0,
            depth: 2.0,
        }
    }
}

/// `1` inside `floor`, easing smoothly to `0` at `radius`.
pub fn profile(distance: f32, floor: f32, radius: f32) -> f32 {
    1.0 - smoothstep(floor, radius, distance)
}

/// Per-sample carve state, combined with `max`.
struct CarveField {
    depth: Vec<f32>,
    weight: Vec<f32>,
    rim: Vec<f32>,
}

impl CarveField {
    fn new(len: usize) -> Self {
        Self {
            depth: vec![0.0; len],
            weight: vec![0.0; len],
            rim: vec![0.0; len],
        }
    }

    fn add(&mut self, idx: usize, weight: f32, depth: f32) {
        self.weight[idx] = self.weight[idx].max(weight);
        self.depth[idx] = self.depth[idx].max(weight * depth);
    }
}

fn crater(
    field: &mut CarveField,
    grid: &Heightfield,
    noise: &Perlin,
    centre: Vec2,
    radius: f32,
    cfg: &CraterConfig,
) {
    let wobble = cfg.wobble.clamp(0.0, 0.9);
    let reach = radius * (1.0 + wobble) + 3.0 * cfg.rim_width;
    let Some((is, js)) = grid.index_range(centre - Vec2::splat(reach), centre + Vec2::splat(reach))
    else {
        return;
    };
    let floor_fraction = cfg.floor_fraction.clamp(0.0, 0.99);
    for j in js {
        for i in is.clone() {
            let offset = grid.position(i, j) - centre;
            let d = offset.length();
            if d > reach {
                continue;
            }
            // Noise sampled on a small circle around the centre, by angle,
            // so the outline is a smooth closed blob.
            let around = offset.normalize_or_zero() * 1.5 + centre * 0.37;
            let radius = radius * (1.0 + wobble * noise.sample(around));
            let floor = radius * floor_fraction;
            let idx = grid.index(i, j);
            field.add(idx, profile(d, floor, radius), cfg.depth);
            if cfg.rim_width > 0.0 {
                let x = (d - radius) / cfg.rim_width;
                let rim = cfg.rim_height * (-x * x).exp();
                field.rim[idx] = field.rim[idx].max(rim);
            }
        }
    }
}

/// Carves a valley along `points`; `half_width(k, t)` gives the half width
/// at fraction `t` of segment `k`.
fn carve_polyline(
    field: &mut CarveField,
    grid: &Heightfield,
    points: &[Vec2],
    max_half: f32,
    floor_fraction: f32,
    depth: f32,
    half_width: impl Fn(usize, f32) -> f32,
) {
    let floor_fraction = floor_fraction.clamp(0.0, 0.99);
    for k in 0..points.len().saturating_sub(1) {
        let (a, b) = (points[k], points[k + 1]);
        let lo = a.min(b) - Vec2::splat(max_half);
        let hi = a.max(b) + Vec2::splat(max_half);
        let Some((is, js)) = grid.index_range(lo, hi) else {
            continue;
        };
        let ab = b - a;
        let len2 = ab.length_squared().max(1e-8);
        for j in js {
            for i in is.clone() {
                let p = grid.position(i, j);
                let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
                let d = p.distance(a + ab * t);
                let r = half_width(k, t);
                if d >= r {
                    continue;
                }
                field.add(grid.index(i, j), profile(d, r * floor_fraction, r), depth);
            }
        }
    }
}

fn valleys(field: &mut CarveField, grid: &Heightfield, trees: &[PathTree], cfg: &ValleyConfig) {
    let half_width = |remaining: f32| {
        let t = smoothstep(0.0, cfg.widen_distance, remaining);
        0.5 * (cfg.width_at_objective + (cfg.width_at_spawn - cfg.width_at_objective) * t)
    };
    let max_half = 0.5 * cfg.width_at_spawn.max(cfg.width_at_objective);
    for route in trees.iter().flat_map(|t| t.routes.iter()) {
        let rem = &route.remaining;
        carve_polyline(
            field,
            grid,
            &route.points,
            max_half,
            cfg.floor_fraction,
            cfg.depth,
            |k, t| half_width(rem[k] + (rem[k + 1] - rem[k]) * t),
        );
    }
}

/// Applies craters and valleys to the base terrain.
pub fn carve(
    base: &BaseTerrain,
    noise: &Perlin,
    objectives: &[Objective],
    spawns: &[SpawnPoint],
    trees: &[PathTree],
    ring_road: &[Vec2],
    config: &MapConfig,
) -> Heightfield {
    let grid = &base.detail;
    let mut field = CarveField::new(grid.data.len());
    let crater_cfg = &config.crater;
    for o in objectives {
        crater(
            &mut field,
            grid,
            noise,
            o.pos,
            crater_cfg.objective_radius,
            crater_cfg,
        );
    }
    for s in spawns {
        crater(
            &mut field,
            grid,
            noise,
            s.pos,
            crater_cfg.spawn_radius,
            crater_cfg,
        );
    }
    valleys(&mut field, grid, trees, &config.valley);
    if config.ring.enabled {
        let half = 0.5 * config.ring.width;
        let floor = config.valley.floor_fraction;
        carve_polyline(
            &mut field,
            grid,
            ring_road,
            half,
            floor,
            config.ring.depth,
            |_, _| half,
        );
    }

    let smoothing = config.valley.floor_smoothing.clamp(0.0, 1.0);
    let mut out = grid.clone();
    for (idx, h) in out.data.iter_mut().enumerate() {
        let w = field.weight[idx];
        let floor = *h + (base.smooth.data[idx] - *h) * w * smoothing;
        // Rims fade where anything is carved, so valleys break through them.
        let rim = field.rim[idx] * (1.0 - w);
        *h = (floor - field.depth[idx] + rim).clamp(0.0, config.max_height);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_has_a_flat_floor_and_soft_edge() {
        assert_eq!(profile(0.0, 2.0, 5.0), 1.0);
        assert_eq!(profile(2.0, 2.0, 5.0), 1.0);
        assert_eq!(profile(5.0, 2.0, 5.0), 0.0);
        let mid = profile(3.5, 2.0, 5.0);
        assert!(mid > 0.0 && mid < 1.0);
    }
}

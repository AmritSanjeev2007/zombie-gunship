//! Placing objectives (toward the centre) and zombie spawns (along the edges).

use glam::Vec2;
use rand::Rng;

use super::Edge;

/// Something the zombies are trying to capture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Objective {
    pub pos: Vec2,
}

/// Where zombies enter the map, and the edge they enter from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpawnPoint {
    pub pos: Vec2,
    pub edge: Edge,
}

/// Rules for where objectives and spawns may go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacementConfig {
    /// Objectives stay within this fraction of the half-extents.
    pub objective_spread: f32,
    /// Bias toward the centre: radius is `spread · uᵏ`. 1 is uniform in
    /// radius; higher packs objectives tighter.
    pub objective_bias: f32,
    /// Distance of spawns from their edge.
    pub spawn_inset: f32,
    /// Random extra inset, so spawns don't sit on a perfect line.
    pub spawn_inset_jitter: f32,
    /// Spawns keep at least this far from the corners.
    pub corner_margin: f32,
    /// Minimum distance between any two points.
    pub min_spacing: f32,
}

impl Default for PlacementConfig {
    fn default() -> Self {
        Self {
            objective_spread: 0.55,
            objective_bias: 2.0,
            spawn_inset: 9.0,
            spawn_inset_jitter: 4.0,
            corner_margin: 14.0,
            min_spacing: 16.0,
        }
    }
}

const ATTEMPTS: usize = 64;

fn far_enough(p: Vec2, others: impl IntoIterator<Item = Vec2>, spacing: f32) -> bool {
    others.into_iter().all(|o| o.distance(p) >= spacing)
}

/// Samples `count` objectives that cluster toward the centre. Spacing is
/// relaxed step by step if the map is too crowded to honour it.
pub fn place_objectives(
    rng: &mut impl Rng,
    size: Vec2,
    count: usize,
    cfg: &PlacementConfig,
) -> Vec<Objective> {
    let extent = size * 0.5 * cfg.objective_spread.clamp(0.0, 1.0);
    let mut placed: Vec<Objective> = Vec::with_capacity(count);
    let mut spacing = cfg.min_spacing;
    while placed.len() < count {
        let mut found = None;
        for _ in 0..ATTEMPTS {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let radius = rng.random::<f32>().powf(cfg.objective_bias.max(0.1));
            let p = Vec2::from_angle(angle) * radius * extent;
            if far_enough(p, placed.iter().map(|o| o.pos), spacing) {
                found = Some(p);
                break;
            }
        }
        match found {
            Some(pos) => placed.push(Objective { pos }),
            None => spacing *= 0.8,
        }
    }
    placed
}

/// Samples `count` spawns. Edges are dealt round-robin from a random start
/// so every edge gets a fair share, then each spawn is placed along its edge.
pub fn place_spawns(
    rng: &mut impl Rng,
    size: Vec2,
    count: usize,
    objectives: &[Objective],
    cfg: &PlacementConfig,
) -> Vec<SpawnPoint> {
    let first = rng.random_range(0..Edge::ALL.len());
    let mut placed: Vec<SpawnPoint> = Vec::with_capacity(count);
    let mut spacing = cfg.min_spacing;
    let mut i = 0;
    while placed.len() < count {
        let edge = Edge::ALL[(first + i) % Edge::ALL.len()];
        let half_len = edge.length(size) * 0.5;
        let margin = cfg.corner_margin.min(half_len * 0.9);
        let mut found = None;
        for _ in 0..ATTEMPTS {
            let along = rng.random_range(-half_len + margin..=half_len - margin);
            let inset = cfg.spawn_inset + rng.random::<f32>() * cfg.spawn_inset_jitter;
            let inset = inset.min(edge.distance(size) * 0.9);
            let p = edge.normal() * (edge.distance(size) - inset) + edge.tangent() * along;
            let others = placed
                .iter()
                .map(|s| s.pos)
                .chain(objectives.iter().map(|o| o.pos));
            if far_enough(p, others, spacing) {
                found = Some(p);
                break;
            }
        }
        match found {
            Some(pos) => {
                placed.push(SpawnPoint { pos, edge });
                i += 1;
            }
            None => spacing *= 0.8,
        }
    }
    placed
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn spawns_hug_their_own_edge_and_objectives_stay_central() {
        let size = Vec2::new(160.0, 120.0);
        let cfg = PlacementConfig::default();
        for seed in 0..50 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let objectives = place_objectives(&mut rng, size, 4, &cfg);
            let spawns = place_spawns(&mut rng, size, 10, &objectives, &cfg);
            for o in &objectives {
                let n = o.pos / (size * 0.5);
                assert!(n.length() <= cfg.objective_spread + 1e-4);
            }
            for s in &spawns {
                let to_edge = s.edge.distance(size) - s.pos.dot(s.edge.normal());
                let max_inset = cfg.spawn_inset + cfg.spawn_inset_jitter;
                assert!((cfg.spawn_inset - 1e-3..=max_inset + 1e-3).contains(&to_edge));
            }
            let all: Vec<Vec2> = objectives
                .iter()
                .map(|o| o.pos)
                .chain(spawns.iter().map(|s| s.pos))
                .collect();
            for (a, p) in all.iter().enumerate() {
                for q in &all[a + 1..] {
                    assert!(p.distance(*q) >= cfg.min_spacing - 1e-3, "seed {seed}");
                }
            }
            // Round-robin: edge counts differ by at most one.
            let counts = Edge::ALL.map(|e| spawns.iter().filter(|s| s.edge == e).count());
            assert!(counts.iter().max().unwrap() - counts.iter().min().unwrap() <= 1);
        }
    }
}

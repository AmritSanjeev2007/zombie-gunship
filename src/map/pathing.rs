//! Zombie paths: one tree per edge, rooted at an objective, with a leaf at
//! every spawn on that edge, turned into smooth routes.

use glam::Vec2;
use rand::Rng;

use super::terrain::{Heightfield, Perlin};
use super::{Edge, Objective, SpawnPoint};

/// Settings for path trees and their curves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathConfig {
    /// Most links from objective to spawn. 1 joins every spawn directly.
    pub max_depth: usize,
    /// Start every tree with a single trunk out of the objective (needs
    /// `max_depth` of 3 or more), so paths merge before they arrive.
    pub root_trunk: bool,
    /// Smallest and largest number of groups a branch splits into.
    pub min_split: usize,
    pub max_split: usize,
    /// `cos` of the widest angle between an objective's direction and an
    /// edge's normal for it to serve that edge. cos 45° matches the
    /// triangles made by the map's diagonals.
    pub target_threshold: f32,
    /// Sideways wobble of branch nodes, as a fraction of the link length.
    pub node_jitter: f32,
    /// Branch nodes go to the lowest of this many candidates.
    pub valley_candidates: usize,
    /// Distance between route samples.
    pub sample_step: f32,
    /// Peak sideways meander of routes.
    pub meander_amplitude: f32,
    /// Meander wiggles per world unit.
    pub meander_frequency: f32,
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            root_trunk: true,
            min_split: 2,
            max_split: 3,
            target_threshold: std::f32::consts::FRAC_1_SQRT_2,
            node_jitter: 0.2,
            valley_candidates: 6,
            sample_step: 1.0,
            meander_amplitude: 2.5,
            meander_frequency: 1.0 / 25.0,
        }
    }
}

/// A point in a path tree. Node 0 is the root, at the objective.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathNode {
    pub pos: Vec2,
    /// Next node toward the objective; `None` for the root.
    pub parent: Option<usize>,
    /// Links from the root.
    pub depth: usize,
    /// Set on leaves: the spawn this node sits on.
    pub spawn: Option<usize>,
}

/// A smooth path from a spawn to the objective, sampled as a polyline.
#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub spawn: usize,
    /// From the spawn (first) to the objective (last).
    pub points: Vec<Vec2>,
    /// Distance along the route still to go, for each point.
    pub remaining: Vec<f32>,
}

impl Route {
    pub fn length(&self) -> f32 {
        self.remaining.first().copied().unwrap_or(0.0)
    }
}

/// All paths from one edge's spawns to that edge's target objective.
#[derive(Clone, Debug, PartialEq)]
pub struct PathTree {
    pub edge: Edge,
    pub objective: usize,
    pub nodes: Vec<PathNode>,
    pub routes: Vec<Route>,
}

impl PathTree {
    /// Links on the longest objective-to-spawn chain.
    pub fn depth(&self) -> usize {
        self.nodes.iter().map(|n| n.depth).max().unwrap_or(0)
    }
}

/// Objective indices sorted by angle from `+X`. Joined in order (and back to
/// the first) they form a single loop.
pub fn objective_ring(objectives: &[Objective]) -> Vec<usize> {
    let angle = |i: usize| {
        let p = objectives[i].pos;
        p.y.atan2(p.x).rem_euclid(std::f32::consts::TAU)
    };
    let mut order: Vec<usize> = (0..objectives.len()).collect();
    order.sort_by(|&a, &b| angle(a).total_cmp(&angle(b)));
    order
}

/// Smooth road through the objectives in ring order, sampled as a polyline.
/// Closed (last point equals the first) for three or more objectives, a
/// single curve for two, and empty for fewer. The curve passes through every
/// objective, with Catmull-Rom tangents and the same meander as the routes.
pub fn ring_road(
    noise: &Perlin,
    objectives: &[Objective],
    ring: &[usize],
    cfg: &PathConfig,
) -> Vec<Vec2> {
    let pts: Vec<Vec2> = ring.iter().map(|&i| objectives[i].pos).collect();
    let n = pts.len();
    if n < 2 {
        return Vec::new();
    }
    let closed = n >= 3;
    let tangent = |i: usize| {
        if closed {
            (pts[(i + 1) % n] - pts[(i + n - 1) % n]).normalize_or_zero()
        } else {
            (pts[1] - pts[0]).normalize_or_zero()
        }
    };
    let segments = if closed { n } else { 1 };
    let mut out = vec![pts[0]];
    for i in 0..segments {
        let j = (i + 1) % n;
        let start = out.len() - 1;
        hermite(
            &mut out,
            pts[i],
            pts[j],
            tangent(i),
            tangent(j),
            cfg.sample_step,
        );
        meander(noise, &mut out[start..], cfg);
    }
    out
}

/// Pushes the inner points of `points` sideways by noise sampled at each
/// point, fading to nothing at both ends so the endpoints stay put.
fn meander(noise: &Perlin, points: &mut [Vec2], cfg: &PathConfig) {
    let len = points.len();
    if len < 3 {
        return;
    }
    let mut remaining = vec![0.0; len];
    for i in (0..len - 1).rev() {
        remaining[i] = remaining[i + 1] + points[i].distance(points[i + 1]);
    }
    let total = remaining[0];
    let fade_len = 12.0;
    let original = points.to_vec();
    for i in 1..len - 1 {
        let dir = (original[i + 1] - original[i - 1]).normalize_or_zero();
        let travelled = total - remaining[i];
        let fade = smoothstep(0.0, fade_len, travelled) * smoothstep(0.0, fade_len, remaining[i]);
        let n = noise.sample(original[i] * cfg.meander_frequency + Vec2::new(311.7, 127.1));
        points[i] = original[i] + dir.perp() * n * cfg.meander_amplitude * fade;
    }
}

/// Picks the objective an edge's zombies head for. Directions are measured
/// in normalised map space, so the regions are the diagonal triangles even
/// on a non-square map.
pub fn choose_target(
    rng: &mut impl Rng,
    edge: Edge,
    size: Vec2,
    objectives: &[Objective],
    threshold: f32,
) -> usize {
    let score = |o: &Objective| {
        (o.pos / (size * 0.5))
            .normalize_or_zero()
            .dot(edge.normal())
    };
    let candidates: Vec<usize> = (0..objectives.len())
        .filter(|&i| score(&objectives[i]) >= threshold)
        .collect();
    if candidates.is_empty() {
        (0..objectives.len())
            .max_by(|&a, &b| score(&objectives[a]).total_cmp(&score(&objectives[b])))
            .unwrap_or(0)
    } else {
        candidates[rng.random_range(0..candidates.len())]
    }
}

/// Builds a path tree for every edge that has spawns.
pub fn build_paths(
    rng: &mut impl Rng,
    noise: &Perlin,
    base: &Heightfield,
    size: Vec2,
    objectives: &[Objective],
    spawns: &[SpawnPoint],
    cfg: &PathConfig,
) -> Vec<PathTree> {
    if objectives.is_empty() {
        return Vec::new();
    }
    let mut trees = Vec::new();
    for edge in Edge::ALL {
        let mut group: Vec<usize> = (0..spawns.len())
            .filter(|&s| spawns[s].edge == edge)
            .collect();
        if group.is_empty() {
            continue;
        }
        group.sort_by(|&a, &b| {
            let t = |s: usize| spawns[s].pos.dot(edge.tangent());
            t(a).total_cmp(&t(b))
        });
        let objective = choose_target(rng, edge, size, objectives, cfg.target_threshold);
        let mut builder = TreeBuilder {
            rng: &mut *rng,
            base,
            size,
            spawns,
            cfg,
            nodes: vec![PathNode {
                pos: objectives[objective].pos,
                parent: None,
                depth: 0,
                spawn: None,
            }],
        };
        builder.expand(0, &group);
        let nodes = builder.nodes;
        let routes = build_routes(noise, &nodes, cfg);
        trees.push(PathTree {
            edge,
            objective,
            nodes,
            routes,
        });
    }
    trees
}

struct TreeBuilder<'a, R: Rng> {
    rng: &'a mut R,
    base: &'a Heightfield,
    size: Vec2,
    spawns: &'a [SpawnPoint],
    cfg: &'a PathConfig,
    nodes: Vec<PathNode>,
}

impl<R: Rng> TreeBuilder<'_, R> {
    fn push(&mut self, pos: Vec2, parent: usize, spawn: Option<usize>) -> usize {
        let depth = self.nodes[parent].depth + 1;
        self.nodes.push(PathNode {
            pos,
            parent: Some(parent),
            depth,
            spawn,
        });
        self.nodes.len() - 1
    }

    /// Connects the (edge-sorted) spawns in `group` below node `parent`.
    fn expand(&mut self, parent: usize, group: &[usize]) {
        let remaining = self
            .cfg
            .max_depth
            .max(1)
            .saturating_sub(self.nodes[parent].depth);
        if remaining <= 1 {
            for &s in group {
                self.push(self.spawns[s].pos, parent, Some(s));
            }
            return;
        }

        // Enough groups that the final level isn't a huge fan.
        let needed = (group.len() as f32).powf(1.0 / remaining as f32).ceil() as usize;
        let lo = self.cfg.min_split.max(1);
        let hi = self.cfg.max_split.max(lo);
        let k = if parent == 0 && self.cfg.root_trunk && group.len() > 1 && remaining >= 3 {
            1
        } else {
            self.rng.random_range(lo..=hi).max(needed).min(group.len())
        };

        for part in split_contiguous(self.rng, group, k) {
            if let [s] = part {
                self.push(self.spawns[*s].pos, parent, Some(*s));
            } else {
                let pos = self.branch_position(parent, part, remaining);
                let node = self.push(pos, parent, None);
                self.expand(node, part);
            }
        }
    }

    /// Partway from the parent toward the group's centre, nudged sideways,
    /// then moved to the lowest nearby ground.
    fn branch_position(&mut self, parent: usize, group: &[usize], remaining: usize) -> Vec2 {
        let from = self.nodes[parent].pos;
        let centre = group.iter().map(|&s| self.spawns[s].pos).sum::<Vec2>() / group.len() as f32;
        let link = centre - from;
        let len = link.length().max(1e-3);
        let along = (self.rng.random_range(0.8..1.25) / remaining as f32).clamp(0.2, 0.75);
        let side = self.rng.random_range(-1.0..=1.0) * self.cfg.node_jitter;
        let guess = from + link * along + link.perp() * side;

        let search = 0.15 * len;
        let half = self.size * 0.5 - Vec2::splat(4.0);
        let mut best = guess.clamp(-half, half);
        let mut best_h = self.base.sample(best);
        for _ in 1..self.cfg.valley_candidates.max(1) {
            let offset = Vec2::from_angle(self.rng.random_range(0.0..std::f32::consts::TAU))
                * search
                * self.rng.random::<f32>().sqrt();
            let p = (guess + offset).clamp(-half, half);
            let h = self.base.sample(p);
            if h < best_h {
                best = p;
                best_h = h;
            }
        }
        best
    }
}

/// Splits `items` into `k` non-empty runs at random cut points.
fn split_contiguous<'a>(rng: &mut impl Rng, items: &'a [usize], k: usize) -> Vec<&'a [usize]> {
    let k = k.clamp(1, items.len().max(1));
    let mut cuts = rand::seq::index::sample(rng, items.len() - 1, k - 1)
        .into_iter()
        .map(|c| c + 1)
        .collect::<Vec<_>>();
    cuts.sort_unstable();
    let mut parts = Vec::with_capacity(k);
    let mut start = 0;
    for cut in cuts.into_iter().chain([items.len()]) {
        parts.push(&items[start..cut]);
        start = cut;
    }
    parts
}

/// Direction of travel (toward the objective) at each node. Every route
/// through a link uses the same tangents, so shared trunks coincide.
fn node_tangents(nodes: &[PathNode]) -> Vec<Vec2> {
    let mut child_sum = vec![Vec2::ZERO; nodes.len()];
    let mut child_count = vec![0usize; nodes.len()];
    for node in nodes {
        if let Some(p) = node.parent {
            child_sum[p] += node.pos;
            child_count[p] += 1;
        }
    }
    nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let behind = if child_count[i] > 0 {
                child_sum[i] / child_count[i] as f32
            } else {
                node.pos
            };
            let ahead = node.parent.map_or(node.pos, |p| nodes[p].pos);
            (ahead - behind).normalize_or_zero()
        })
        .collect()
}

/// Cubic Hermite from `a` to `b` with unit tangents `ta`, `tb`, appended to
/// `out` without its first point.
fn hermite(out: &mut Vec<Vec2>, a: Vec2, b: Vec2, ta: Vec2, tb: Vec2, step: f32) {
    let len = a.distance(b);
    let (ma, mb) = (ta * len, tb * len);
    let n = (len / step.max(0.05)).ceil().max(1.0) as usize;
    for k in 1..=n {
        let t = k as f32 / n as f32;
        let (t2, t3) = (t * t, t * t * t);
        let p = a * (2.0 * t3 - 3.0 * t2 + 1.0)
            + ma * (t3 - 2.0 * t2 + t)
            + b * (-2.0 * t3 + 3.0 * t2)
            + mb * (t3 - t2);
        out.push(p);
    }
}

fn build_routes(noise: &Perlin, nodes: &[PathNode], cfg: &PathConfig) -> Vec<Route> {
    let tangents = node_tangents(nodes);
    let mut routes = Vec::new();
    for (leaf, node) in nodes.iter().enumerate() {
        let Some(spawn) = node.spawn else { continue };

        let mut points = vec![node.pos];
        let mut at = leaf;
        while let Some(parent) = nodes[at].parent {
            let (a, b) = (nodes[at].pos, nodes[parent].pos);
            hermite(
                &mut points,
                a,
                b,
                tangents[at],
                tangents[parent],
                cfg.sample_step,
            );
            at = parent;
        }

        // Distance still to travel, measured before the meander.
        let mut remaining = vec![0.0; points.len()];
        for i in (0..points.len().saturating_sub(1)).rev() {
            remaining[i] = remaining[i + 1] + points[i].distance(points[i + 1]);
        }

        // Meander from noise sampled at each point, so shared trunks wiggle
        // identically.
        meander(noise, &mut points, cfg);

        routes.push(Route {
            spawn,
            points,
            remaining,
        });
    }
    routes
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn ring_is_sorted_by_angle() {
        let objectives = [
            Vec2::new(0.0, -1.0),
            Vec2::new(1.0, 0.1),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 1.0),
        ]
        .map(|pos| Objective { pos });
        assert_eq!(objective_ring(&objectives), vec![1, 3, 2, 0]);
    }

    #[test]
    fn target_prefers_the_edge_triangle_and_falls_back() {
        let size = Vec2::new(100.0, 50.0);
        let mut rng = ChaCha8Rng::seed_from_u64(3);
        // (40, 15) is in the east triangle once normalised by the extents.
        let objectives =
            [Vec2::new(40.0, 15.0), Vec2::new(-5.0, -20.0)].map(|pos| Objective { pos });
        assert_eq!(
            choose_target(&mut rng, Edge::East, size, &objectives, 0.7),
            0
        );
        assert_eq!(
            choose_target(&mut rng, Edge::North, size, &objectives, 0.7),
            1
        );
        // Nothing qualifies for the west: fall back to the best score.
        assert_eq!(
            choose_target(&mut rng, Edge::West, size, &objectives, 0.99),
            1
        );
    }

    #[test]
    fn splits_are_contiguous_and_complete() {
        let items: Vec<usize> = (0..7).collect();
        let mut rng = ChaCha8Rng::seed_from_u64(9);
        for k in 1..=7 {
            let parts = split_contiguous(&mut rng, &items, k);
            assert_eq!(parts.len(), k);
            assert!(parts.iter().all(|p| !p.is_empty()));
            assert_eq!(parts.concat(), items);
        }
    }
}

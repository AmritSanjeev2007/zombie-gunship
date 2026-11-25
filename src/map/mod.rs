//! Procedural battlefield generation.
//!
//! [`generate`] turns a [`MapConfig`] into a [`Map`]: a heightfield with
//! craters at the objectives and zombie spawns, and valleys along the paths
//! zombies will follow. It is pure CPU work with no GL or ECS access, so the
//! same config always yields the same map. See `plan.md` for the procedure.

pub mod apron;
pub mod carve;
pub mod pathing;
pub mod points;
pub mod terrain;

use glam::{Vec2, Vec3};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub use apron::{Apron, ApronConfig};
pub use carve::{CraterConfig, RingConfig, ValleyConfig};
pub use pathing::{PathConfig, PathTree, Route};
pub use points::{Objective, PlacementConfig, SpawnPoint};
pub use terrain::{Heightfield, NoiseConfig};

/// One side of the map rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Edge {
    /// The `+X` side.
    East,
    /// The `+Z` side.
    South,
    /// The `-X` side.
    West,
    /// The `-Z` side.
    North,
}

impl Edge {
    pub const ALL: [Edge; 4] = [Edge::East, Edge::South, Edge::West, Edge::North];

    /// Unit vector pointing out of the map through this edge (XZ plane).
    pub fn normal(self) -> Vec2 {
        match self {
            Edge::East => Vec2::X,
            Edge::South => Vec2::Y,
            Edge::West => Vec2::NEG_X,
            Edge::North => Vec2::NEG_Y,
        }
    }

    /// Unit vector running along this edge.
    pub fn tangent(self) -> Vec2 {
        self.normal().perp()
    }

    /// Length of this edge for a map of the given size.
    pub fn length(self, size: Vec2) -> f32 {
        match self {
            Edge::East | Edge::West => size.y,
            Edge::South | Edge::North => size.x,
        }
    }

    /// Distance from the map centre to this edge.
    pub fn distance(self, size: Vec2) -> f32 {
        match self {
            Edge::East | Edge::West => size.x * 0.5,
            Edge::South | Edge::North => size.y * 0.5,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Edge::East => "East (+X)",
            Edge::South => "South (+Z)",
            Edge::West => "West (-X)",
            Edge::North => "North (-Z)",
        }
    }
}

/// Everything that decides how a map looks. Sizes are in world units.
#[derive(Clone, Debug, PartialEq)]
pub struct MapConfig {
    pub seed: u64,
    /// Width (X) and depth (Z) of the map rectangle.
    pub size: Vec2,
    pub max_height: f32,
    /// Heightfield samples per world unit.
    pub resolution: f32,
    pub objective_count: usize,
    pub spawn_count: usize,
    pub noise: NoiseConfig,
    pub placement: PlacementConfig,
    pub path: PathConfig,
    pub crater: CraterConfig,
    pub valley: ValleyConfig,
    pub ring: RingConfig,
    pub apron: ApronConfig,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            size: Vec2::new(160.0, 160.0),
            max_height: 14.0,
            resolution: 1.25,
            objective_count: 3,
            spawn_count: 8,
            noise: NoiseConfig::default(),
            placement: PlacementConfig::default(),
            path: PathConfig::default(),
            crater: CraterConfig::default(),
            valley: ValleyConfig::default(),
            ring: RingConfig::default(),
            apron: ApronConfig::default(),
        }
    }
}

impl MapConfig {
    /// Heightfield samples along X and Z.
    pub fn grid_size(&self) -> (usize, usize) {
        let samples = |len: f32| ((len * self.resolution).round() as usize + 1).clamp(2, 1025);
        (samples(self.size.x), samples(self.size.y))
    }
}

/// A generated battlefield.
#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub config: MapConfig,
    /// Final, carved terrain heights.
    pub heights: Heightfield,
    pub objectives: Vec<Objective>,
    pub spawns: Vec<SpawnPoint>,
    /// Objective indices sorted by angle from `+X`, forming a closed loop.
    pub objective_ring: Vec<usize>,
    /// Smooth road through the objectives in ring order (closed for three or
    /// more). Carved as a valley when `config.ring.enabled`.
    pub ring_road: Vec<Vec2>,
    /// One path tree per edge that has spawns.
    pub paths: Vec<PathTree>,
    /// Land around the map, flattening into a plain.
    pub apron: Apron,
}

impl Map {
    /// Height of the terrain at a world XZ position.
    pub fn height_at(&self, p: Vec2) -> f32 {
        self.heights.sample(p)
    }

    /// Lifts an XZ position onto the terrain surface.
    pub fn surface(&self, p: Vec2) -> Vec3 {
        Vec3::new(p.x, self.height_at(p), p.y)
    }

    /// Every route, across all edges.
    pub fn routes(&self) -> impl Iterator<Item = &Route> {
        self.paths.iter().flat_map(|tree| tree.routes.iter())
    }
}

/// Runs the whole pipeline described in `plan.md`.
pub fn generate(config: &MapConfig) -> Map {
    let mut rng = ChaCha8Rng::seed_from_u64(config.seed);
    let noise = terrain::Perlin::new(&mut rng);
    let (cols, rows) = config.grid_size();
    let base = terrain::base_terrain(
        &noise,
        &config.noise,
        cols,
        rows,
        config.size,
        config.max_height,
    );

    let objectives = points::place_objectives(
        &mut rng,
        config.size,
        config.objective_count,
        &config.placement,
    );
    let spawns = points::place_spawns(
        &mut rng,
        config.size,
        config.spawn_count,
        &objectives,
        &config.placement,
    );
    let objective_ring = pathing::objective_ring(&objectives);
    let ring_road = pathing::ring_road(&noise, &objectives, &objective_ring, &config.path);
    let paths = pathing::build_paths(
        &mut rng,
        &noise,
        &base.detail,
        config.size,
        &objectives,
        &spawns,
        &config.path,
    );

    let heights = carve::carve(
        &base,
        &noise,
        &objectives,
        &spawns,
        &paths,
        &ring_road,
        config,
    );
    let apron = apron::build(
        &heights,
        &noise,
        &config.noise,
        config.max_height,
        &config.apron,
    );

    Map {
        config: config.clone(),
        heights,
        objectives,
        spawns,
        objective_ring,
        ring_road,
        paths,
        apron,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> MapConfig {
        MapConfig {
            resolution: 0.6,
            ..Default::default()
        }
    }

    #[test]
    fn same_seed_gives_same_map() {
        let cfg = small();
        assert_eq!(generate(&cfg), generate(&cfg));
        let other = generate(&MapConfig {
            seed: 2,
            ..cfg.clone()
        });
        assert_ne!(generate(&cfg).heights, other.heights);
    }

    #[test]
    fn counts_and_routes_match_config() {
        for seed in 0..20 {
            for spawn_count in [1, 3, 8, 15] {
                let cfg = MapConfig {
                    seed,
                    spawn_count,
                    ..small()
                };
                let map = generate(&cfg);
                assert_eq!(map.objectives.len(), cfg.objective_count);
                assert_eq!(map.spawns.len(), spawn_count);
                assert_eq!(map.objective_ring.len(), cfg.objective_count);

                let mut seen = vec![0; spawn_count];
                for tree in &map.paths {
                    assert!(tree.depth() <= cfg.path.max_depth, "seed {seed}");
                    for route in &tree.routes {
                        seen[route.spawn] += 1;
                        let spawn = map.spawns[route.spawn];
                        assert_eq!(spawn.edge, tree.edge);
                        let first = route.points[0];
                        let last = *route.points.last().unwrap();
                        assert!(first.distance(spawn.pos) < 1e-3);
                        let goal = map.objectives[tree.objective].pos;
                        assert!(last.distance(goal) < 1e-3);
                    }
                }
                assert!(seen.iter().all(|&n| n == 1), "seed {seed}: {seen:?}");
            }
        }
    }

    #[test]
    fn ring_road_visits_every_objective_and_closes() {
        for count in 1..=6 {
            let map = generate(&MapConfig {
                objective_count: count,
                ..small()
            });
            let road = &map.ring_road;
            match count {
                1 => assert!(road.is_empty()),
                2 => assert!(road.first() != road.last()),
                _ => assert!(road[0].distance(*road.last().unwrap()) < 1e-4),
            }
            for o in &map.objectives {
                if count > 1 {
                    assert!(road.iter().any(|p| p.distance(o.pos) < 1e-3));
                }
            }
        }
    }

    #[test]
    fn objectives_sit_in_craters() {
        let cfg = small();
        let mut rng = ChaCha8Rng::seed_from_u64(cfg.seed);
        let noise = terrain::Perlin::new(&mut rng);
        let (cols, rows) = cfg.grid_size();
        let base = terrain::base_terrain(&noise, &cfg.noise, cols, rows, cfg.size, cfg.max_height);
        let map = generate(&cfg);
        for o in &map.objectives {
            assert!(map.height_at(o.pos) < base.detail.sample(o.pos) - 0.5 * cfg.crater.depth);
        }
        assert!(
            map.heights
                .data
                .iter()
                .all(|h| (0.0..=cfg.max_height).contains(h))
        );
    }
}

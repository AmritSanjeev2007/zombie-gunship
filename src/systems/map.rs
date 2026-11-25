use std::time::Instant;

use glam::Vec3;

use crate::components::{MapMarker, MeshRenderer, Name, Primitive, Terrain, Transform};
use crate::engine::{System, UiContext, UpdateContext};
use crate::map::{self, MapConfig};

/// Owns the map settings. Generates a map on the first frame and whenever
/// asked, publishes it to
/// [`Resources::map`](crate::engine::Resources::map), and keeps marker
/// entities on the objectives and spawns. Its "Map" window (toggle with F1)
/// edits the settings.
pub struct MapSystem {
    pub config: MapConfig,
    regenerate: bool,
    open: bool,
    f1_was_down: bool,
    last_generation_ms: f32,
}

impl Default for MapSystem {
    fn default() -> Self {
        Self {
            config: MapConfig::default(),
            regenerate: true,
            open: true,
            f1_was_down: false,
            last_generation_ms: 0.0,
        }
    }
}

impl MapSystem {
    fn respawn_markers(world: &mut hecs::World, map: &map::Map) {
        let old: Vec<hecs::Entity> = world
            .query::<(hecs::Entity, &MapMarker)>()
            .iter()
            .map(|(e, _)| e)
            .collect();
        for entity in old {
            let _ = world.despawn(entity);
        }
        for (i, objective) in map.objectives.iter().enumerate() {
            world.spawn((
                Name::new(format!("Objective {}", i + 1)),
                MapMarker,
                Transform::from_translation(map.surface(objective.pos) + Vec3::Y * 1.0)
                    .with_scale(Vec3::splat(2.0)),
                MeshRenderer::new(Primitive::Cube, Vec3::new(0.85, 0.75, 0.3)),
            ));
        }
        for (i, spawn) in map.spawns.iter().enumerate() {
            world.spawn((
                Name::new(format!("Zombie spawn {} ({})", i + 1, spawn.edge.label())),
                MapMarker,
                Transform::from_translation(map.surface(spawn.pos) + Vec3::Y * 0.5),
                MeshRenderer::new(Primitive::Cube, Vec3::new(0.45, 0.8, 0.25)),
            ));
        }
    }
}

impl System for MapSystem {
    fn name(&self) -> &'static str {
        "Map"
    }

    fn update(&mut self, ctx: &mut UpdateContext) {
        if !self.regenerate {
            return;
        }
        self.regenerate = false;
        let start = Instant::now();
        let map = map::generate(&self.config);
        self.last_generation_ms = start.elapsed().as_secs_f32() * 1000.0;
        Self::respawn_markers(ctx.world, &map);
        ctx.resources.map = Some(map);
        ctx.resources.map_revision += 1;
    }

    fn ui(&mut self, ctx: &mut UiContext) {
        let f1 = ctx.resources.input.key_down(winit::keyboard::KeyCode::F1);
        if f1 && !self.f1_was_down {
            self.open = !self.open;
        }
        self.f1_was_down = f1;

        let mut open = self.open;
        let mut changed = false;
        let cfg = &mut self.config;
        egui::Window::new("Map")
            .open(&mut open)
            .default_pos([ctx.resources.viewport.width as f32 - 300.0, 12.0])
            .default_width(280.0)
            .resizable(false)
            .show(ctx.egui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("seed");
                    changed |= ui.add(egui::DragValue::new(&mut cfg.seed)).changed();
                    if ui.button("Random").clicked() {
                        cfg.seed = rand::random::<u32>() as u64;
                        changed = true;
                    }
                });
                changed |= count(ui, &mut cfg.objective_count, 1..=8, "objectives");
                changed |= count(ui, &mut cfg.spawn_count, 1..=24, "zombie spawns");

                egui::CollapsingHeader::new("Terrain").show(ui, |ui| {
                    changed |= slider(ui, &mut cfg.size.x, 60.0..=400.0, "width");
                    changed |= slider(ui, &mut cfg.size.y, 60.0..=400.0, "depth");
                    changed |= slider(ui, &mut cfg.max_height, 2.0..=40.0, "max height");
                    changed |= slider(ui, &mut cfg.resolution, 0.25..=3.0, "samples / unit");
                    let mut octaves = cfg.noise.octaves as usize;
                    changed |= count(ui, &mut octaves, 1..=8, "octaves");
                    cfg.noise.octaves = octaves as u32;
                    let mut wavelength = 1.0 / cfg.noise.frequency;
                    changed |= slider(ui, &mut wavelength, 10.0..=200.0, "feature size");
                    cfg.noise.frequency = 1.0 / wavelength;
                    changed |= slider(ui, &mut cfg.noise.persistence, 0.1..=0.9, "persistence");
                    changed |= slider(ui, &mut cfg.noise.lacunarity, 1.5..=3.0, "lacunarity");
                });
                egui::CollapsingHeader::new("Placement").show(ui, |ui| {
                    let p = &mut cfg.placement;
                    changed |= slider(ui, &mut p.objective_spread, 0.0..=1.0, "objective spread");
                    changed |= slider(ui, &mut p.objective_bias, 0.5..=5.0, "centre bias");
                    changed |= slider(ui, &mut p.spawn_inset, 0.0..=40.0, "spawn inset");
                    changed |= slider(ui, &mut p.min_spacing, 0.0..=50.0, "min spacing");
                });
                egui::CollapsingHeader::new("Paths").show(ui, |ui| {
                    let p = &mut cfg.path;
                    changed |= count(ui, &mut p.max_depth, 1..=6, "max depth");
                    changed |= ui.checkbox(&mut p.root_trunk, "single trunk").changed();
                    changed |= count(ui, &mut p.max_split, 2..=5, "max split");
                    p.min_split = p.min_split.min(p.max_split);
                    let mut angle = p.target_threshold.clamp(-1.0, 1.0).acos().to_degrees();
                    changed |= slider(ui, &mut angle, 0.0..=90.0, "target cone °");
                    p.target_threshold = angle.to_radians().cos();
                    changed |= slider(ui, &mut p.node_jitter, 0.0..=0.6, "node jitter");
                    changed |= slider(ui, &mut p.meander_amplitude, 0.0..=8.0, "meander");
                });
                egui::CollapsingHeader::new("Craters & valleys").show(ui, |ui| {
                    let c = &mut cfg.crater;
                    changed |= slider(ui, &mut c.objective_radius, 2.0..=30.0, "objective crater");
                    changed |= slider(ui, &mut c.spawn_radius, 2.0..=20.0, "spawn crater");
                    changed |= slider(ui, &mut c.depth, 0.0..=12.0, "crater depth");
                    changed |= slider(ui, &mut c.rim_height, 0.0..=5.0, "rim height");
                    let v = &mut cfg.valley;
                    changed |= slider(ui, &mut v.depth, 0.0..=12.0, "valley depth");
                    changed |= slider(ui, &mut v.width_at_spawn, 2.0..=30.0, "width at spawn");
                    changed |= slider(
                        ui,
                        &mut v.width_at_objective,
                        2.0..=40.0,
                        "width at objective",
                    );
                    changed |= slider(ui, &mut v.floor_smoothing, 0.0..=1.0, "floor smoothing");
                    let r = &mut cfg.ring;
                    changed |= ui
                        .checkbox(&mut r.enabled, "carve objective ring")
                        .changed();
                    changed |= slider(ui, &mut r.width, 2.0..=30.0, "ring width");
                    changed |= slider(ui, &mut r.depth, 0.0..=12.0, "ring depth");
                });

                ui.separator();
                for terrain in ctx.world.query_mut::<&mut Terrain>() {
                    ui.checkbox(&mut terrain.show_paths, "Show paths overlay");
                }
                ui.horizontal(|ui| {
                    if ui.button("Regenerate").clicked() {
                        changed = true;
                    }
                    if ui.button("Defaults").clicked() {
                        *cfg = MapConfig {
                            seed: cfg.seed,
                            ..Default::default()
                        };
                        changed = true;
                    }
                });
                if let Some(map) = &ctx.resources.map {
                    for tree in &map.paths {
                        let longest = tree.routes.iter().map(|r| r.length()).fold(0.0, f32::max);
                        ui.small(format!(
                            "{}: {} spawns to objective {}, depth {}, longest {:.0} m",
                            tree.edge.label(),
                            tree.routes.len(),
                            tree.objective + 1,
                            tree.depth(),
                            longest
                        ));
                    }
                    let routes = map.routes().count();
                    ui.small(format!(
                        "{}×{} samples, {} routes, built in {:.0} ms",
                        map.heights.cols, map.heights.rows, routes, self.last_generation_ms
                    ));
                }
            });
        self.open = open;
        if changed {
            self.regenerate = true;
        }
    }
}

fn slider(
    ui: &mut egui::Ui,
    v: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    text: &str,
) -> bool {
    ui.add(egui::Slider::new(v, range).text(text)).changed()
}

fn count(
    ui: &mut egui::Ui,
    v: &mut usize,
    range: std::ops::RangeInclusive<usize>,
    text: &str,
) -> bool {
    ui.add(egui::Slider::new(v, range).text(text)).changed()
}

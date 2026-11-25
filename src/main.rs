//! Zombie Gunship: orbit a battlefield in a gunship and stop the zombies
//! from capturing the objective.
//!
//! Module layout:
//! - `engine`     – the component/system framework (world, resources, schedule).
//! - `components` – plain-data components that get attached to entities.
//! - `systems`    – behaviour: each system reads/writes components each frame.
//! - `map`        – procedural battlefield: terrain, objectives, spawns, paths.
//! - `render`     – thin OpenGL helpers (shaders, textures, meshes).
//! - `platform`   – window, GL context and window icon.
//! - `scene`      – builds the initial world from components and systems.

mod app;
mod assets;
mod components;
mod engine;
mod map;
mod platform;
mod render;
mod scene;
mod systems;

fn main() -> anyhow::Result<()> {
    app::run()
}

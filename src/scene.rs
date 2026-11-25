//! Builds the starting world: which entities exist, which components they
//! carry, and which systems run (and in what order).

use glam::Vec3;

use crate::assets;
use crate::components::{
    Camera, Name, OrbitController, Skybox, Terrain, Transform,
};
use crate::engine::Engine;
use crate::systems::{
    CameraSystem, ClearSystem, DebugUiSystem, MapSystem, MeshRendererSystem, OrbitCameraSystem,
    SkyboxRendererSystem, TerrainRendererSystem,
};

pub fn build(engine: &mut Engine) {
    register_systems(engine);
    spawn_entities(engine);
}

fn register_systems(engine: &mut Engine) {
    engine
        // Simulation
        .add_system(MapSystem::default())
        .add_system(OrbitCameraSystem)
        .add_system(CameraSystem)
        // Rendering (order matters: clear → sky → geometry)
        .add_system(ClearSystem {
            color: [0.05, 0.02, 0.02],
        })
        .add_system(SkyboxRendererSystem::default())
        .add_system(TerrainRendererSystem::default())
        .add_system(MeshRendererSystem::default())
        // UI
        .add_system(DebugUiSystem::default());
}

fn spawn_entities(engine: &mut Engine) {
    let world = &mut engine.world;

    // The gunship's eye: a camera that circles the battlefield.
    world.spawn((
        Name::new("Gunship camera"),
        Transform::default(),
        Camera::default(),
        OrbitController {
            target: Vec3::new(0.0, 4.0, 0.0),
            radius: 140.0,
            elevation: 40f32.to_radians(),
            max_radius: 400.0,
            ..Default::default()
        },
    ));

    world.spawn((
        Name::new("Sky"),
        Skybox::new(assets::path("skybox/red.png")),
    ));

    // The battlefield itself comes from the map system; objective and spawn
    // markers are spawned there on every regeneration.
    world.spawn((Name::new("Terrain"), Terrain::default()));
}

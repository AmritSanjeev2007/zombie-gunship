use glam::{Mat4, Vec3};

use super::{Input, Time};
use crate::map::Map;

/// Size of the drawable area in physical pixels.
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    pub fn aspect(&self) -> f32 {
        self.width.max(1) as f32 / self.height.max(1) as f32
    }
}

/// View and projection of the active camera, computed once per frame by the
/// camera system and read by every renderer.
#[derive(Clone, Copy, Debug)]
pub struct CameraMatrices {
    pub view: Mat4,
    pub projection: Mat4,
    #[allow(dead_code)] // for upcoming lighting/audio systems
    pub position: Vec3,
}

impl CameraMatrices {
    pub fn view_projection(&self) -> Mat4 {
        self.projection * self.view
    }
}

/// Atmosphere around the battlefield. Fog is measured from the map's edge,
/// not from the camera, so the playable area stays clear at any zoom while
/// the land beyond fades into a blanket that meets the sky at the horizon.
#[derive(Clone, Copy, Debug)]
pub struct Fog {
    /// Matches the sky near the horizon, so ground and sky meet seamlessly.
    pub color: Vec3,
    /// Distance beyond the map's edge where fog begins.
    pub start: f32,
    /// Distance beyond the map's edge where fog is total.
    pub end: f32,
    /// Sky below the horizon is fog; it fades back to the sky over this
    /// many radians above it.
    pub sky_blend: f32,
    /// Low-lying mist that pools on the plain outside the map.
    pub ground: bool,
    /// Opacity of the mist at ground level (0..1).
    pub ground_density: f32,
    /// Height of the mist above the plain.
    pub ground_thickness: f32,
    /// Darkens land outside the map so the playable edge reads clearly (0..1).
    pub outside_dim: f32,
}

impl Default for Fog {
    fn default() -> Self {
        Self {
            // Average horizon colour of the default red skybox.
            color: Vec3::new(0.78, 0.71, 0.61),
            start: 10.0,
            end: 280.0,
            sky_blend: 8f32.to_radians(),
            ground: true,
            ground_density: 0.85,
            ground_thickness: 5.0,
            outside_dim: 0.25,
        }
    }
}

/// Shared state that doesn't belong to any single entity.
pub struct Resources {
    pub time: Time,
    pub input: Input,
    pub viewport: Viewport,
    /// `None` until a camera system has run.
    pub camera: Option<CameraMatrices>,
    /// The battlefield. `None` until the map system generates one.
    pub map: Option<Map>,
    /// Bumped whenever `map` is replaced, so renderers know to rebuild.
    pub map_revision: u64,
    pub fog: Fog,
}

impl Resources {
    pub fn new(viewport: Viewport) -> Self {
        Self {
            time: Time::new(),
            input: Input::default(),
            viewport,
            camera: None,
            map: None,
            map_revision: 0,
            fog: Fog::default(),
        }
    }
}

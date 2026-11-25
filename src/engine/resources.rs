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
        }
    }
}

use std::path::PathBuf;

use glam::Vec3;

/// An infinitely distant background, drawn from an equirectangular
/// (2:1 panorama) image. Only the first skybox in the world is drawn.
#[derive(Clone, Debug)]
pub struct Skybox {
    /// Path to the panorama image, usually from [`crate::assets::path`].
    pub texture: PathBuf,
    /// Multiplies the texture colour.
    pub tint: Vec3,
    /// Spins the panorama around the vertical axis, in radians.
    pub rotation: f32,
}

impl Skybox {
    pub fn new(texture: impl Into<PathBuf>) -> Self {
        Self {
            texture: texture.into(),
            tint: Vec3::ONE,
            rotation: 0.0,
        }
    }
}

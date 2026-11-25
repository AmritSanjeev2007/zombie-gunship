use crate::components::{Camera, Transform};
use crate::engine::{CameraMatrices, System, UpdateContext};

/// Finds the first active [`Camera`] and publishes its matrices to
/// [`Resources::camera`](crate::engine::Resources::camera) for the renderers.
/// Register it after anything that moves cameras.
pub struct CameraSystem;

impl System for CameraSystem {
    fn name(&self) -> &'static str {
        "Camera"
    }

    fn update(&mut self, ctx: &mut UpdateContext) {
        let aspect = ctx.resources.viewport.aspect();
        ctx.resources.camera = ctx
            .world
            .query::<(&Camera, &Transform)>()
            .iter()
            .find(|(camera, _)| camera.active)
            .map(|(camera, transform)| CameraMatrices {
                view: transform.matrix().inverse(),
                projection: camera.projection(aspect),
                position: transform.translation,
            });
    }
}

use winit::event::MouseButton;

use crate::components::{OrbitController, Transform};
use crate::engine::{System, UpdateContext};

/// Moves every entity with an [`OrbitController`] around its target.
///
/// Controls: left-drag to orbit by hand (pauses auto-rotation while held),
/// scroll to zoom.
pub struct OrbitCameraSystem;

impl System for OrbitCameraSystem {
    fn name(&self) -> &'static str {
        "Orbit camera"
    }

    fn update(&mut self, ctx: &mut UpdateContext) {
        let dt = ctx.resources.time.delta;
        let input = &ctx.resources.input;
        let dragging = input.button_down(MouseButton::Left);
        let (dx, dy) = input.mouse_delta;
        let scroll = input.scroll_delta;

        for (orbit, transform) in ctx
            .world
            .query_mut::<(&mut OrbitController, &mut Transform)>()
        {
            if dragging {
                orbit.azimuth -= dx * orbit.drag_sensitivity;
                orbit.elevation += dy * orbit.drag_sensitivity;
            } else if orbit.auto_rotate {
                orbit.azimuth += orbit.angular_speed * dt;
            }
            if scroll != 0.0 {
                orbit.radius *= 1.0 - scroll * orbit.zoom_sensitivity;
            }
            orbit.clamp();

            transform.translation = orbit.position();
            transform.look_at(orbit.target, glam::Vec3::Y);
        }
    }
}

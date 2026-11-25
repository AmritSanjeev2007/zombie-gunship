//! The component/system framework.
//!
//! Entities live in a [`hecs::World`] and are just bags of components.
//! Behaviour lives in [`System`]s, which are run in order every frame by the
//! [`Engine`]. Shared, non-entity state (time, input, viewport, the active
//! camera's matrices) lives in [`Resources`].
//!
//! Adding a feature usually means: define a component in `components/`,
//! write a system in `systems/` that queries for it, and register that
//! system in `scene.rs`.

mod input;
mod resources;
mod system;
mod time;

pub use input::Input;
pub use resources::{CameraMatrices, Resources, Viewport};
pub use system::{FrameContext, System, UiContext, UpdateContext};
pub use time::Time;

use hecs::World;

/// Owns the world, the shared resources and the ordered list of systems.
pub struct Engine {
    pub world: World,
    pub resources: Resources,
    systems: Vec<Box<dyn System>>,
}

impl Engine {
    pub fn new(viewport: Viewport) -> Self {
        Self {
            world: World::new(),
            resources: Resources::new(viewport),
            systems: Vec::new(),
        }
    }

    /// Appends a system. Systems run in the order they were added.
    pub fn add_system(&mut self, system: impl System + 'static) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    /// Advances the simulation by one frame.
    pub fn update(&mut self) {
        self.resources.time.tick();
        let mut ctx = UpdateContext {
            world: &mut self.world,
            resources: &mut self.resources,
        };
        for system in &mut self.systems {
            system.update(&mut ctx);
        }
        self.resources.input.end_frame();
    }

    /// Draws the 3D scene.
    pub fn render(&mut self, gl: &glow::Context) {
        let mut ctx = FrameContext {
            gl,
            world: &self.world,
            resources: &self.resources,
        };
        for system in &mut self.systems {
            system.render(&mut ctx);
        }
    }

    /// Lets each system contribute to the immediate-mode UI.
    pub fn ui(&mut self, egui: &egui::Context) {
        let names: Vec<&'static str> = self.systems.iter().map(|s| s.name()).collect();
        let mut ctx = UiContext {
            egui,
            world: &mut self.world,
            resources: &mut self.resources,
            system_names: &names,
        };
        for system in &mut self.systems {
            system.ui(&mut ctx);
        }
    }

    /// Releases GPU resources held by systems.
    pub fn destroy(&mut self, gl: &glow::Context) {
        for system in &mut self.systems {
            system.destroy(gl);
        }
    }
}

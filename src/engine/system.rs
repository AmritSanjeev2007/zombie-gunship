use hecs::World;

use super::Resources;

/// Everything a system may touch while updating the simulation.
pub struct UpdateContext<'a> {
    pub world: &'a mut World,
    pub resources: &'a mut Resources,
}

/// Everything a system may touch while drawing. The world is read-only here.
pub struct FrameContext<'a> {
    pub gl: &'a glow::Context,
    pub world: &'a World,
    pub resources: &'a Resources,
}

/// Everything a system may touch while building the debug UI.
pub struct UiContext<'a> {
    pub egui: &'a egui::Context,
    pub world: &'a mut World,
    pub resources: &'a mut Resources,
    /// Names of all registered systems, in run order.
    pub system_names: &'a [&'static str],
}

/// A unit of behaviour. Every hook is optional, so a system only implements
/// the phases it cares about.
///
/// Frame order: `update` (all systems) → `render` (all systems) → `ui`
/// (all systems). GPU resources should be created lazily in `render` and
/// released in `destroy`.
pub trait System {
    fn name(&self) -> &'static str;

    fn update(&mut self, _ctx: &mut UpdateContext) {}

    fn render(&mut self, _ctx: &mut FrameContext) {}

    fn ui(&mut self, _ctx: &mut UiContext) {}

    fn destroy(&mut self, _gl: &glow::Context) {}
}

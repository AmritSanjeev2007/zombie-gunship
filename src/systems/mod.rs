//! Systems: the behaviour that brings components to life. Each one is a
//! small struct implementing [`crate::engine::System`].

mod camera;
mod clear;
mod debug_ui;
mod map;
mod mesh_renderer;
mod orbit_camera;
mod skybox_renderer;
mod terrain_renderer;

pub use camera::CameraSystem;
pub use clear::ClearSystem;
pub use debug_ui::DebugUiSystem;
pub use map::MapSystem;
pub use mesh_renderer::MeshRendererSystem;
pub use orbit_camera::OrbitCameraSystem;
pub use skybox_renderer::SkyboxRendererSystem;
pub use terrain_renderer::TerrainRendererSystem;

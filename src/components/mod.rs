//! Components: plain data attached to entities. They hold no GPU handles and
//! no behaviour beyond small helpers; systems give them meaning.

mod camera;
mod mesh;
mod name;
mod orbit;
mod skybox;
mod terrain;
mod transform;

pub use camera::Camera;
pub use mesh::{MeshRenderer, Primitive};
pub use name::Name;
pub use orbit::OrbitController;
pub use skybox::Skybox;
pub use terrain::{MapMarker, Terrain};
pub use transform::Transform;

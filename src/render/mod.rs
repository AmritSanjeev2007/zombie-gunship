//! Thin, safe-ish wrappers around the raw OpenGL calls the systems need.

pub mod mesh;
mod shader;
pub mod texture;

pub use mesh::GpuMesh;
pub use shader::ShaderProgram;
pub use texture::Texture;

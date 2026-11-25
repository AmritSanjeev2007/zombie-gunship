use glam::Vec3;

/// Built-in shapes. Real models can be added as another variant later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Primitive {
    /// Unit cube centred on the origin.
    Cube,
    /// Flat square of grid lines on the XZ plane, 1 unit across.
    #[allow(dead_code)] // unused since the terrain replaced the placeholder grid
    Grid,
}

/// Draws a primitive at the entity's transform.
#[derive(Clone, Copy, Debug)]
pub struct MeshRenderer {
    pub primitive: Primitive,
    pub color: Vec3,
    /// Unlit meshes ignore the sun (useful for lines and markers).
    pub lit: bool,
}

impl MeshRenderer {
    pub fn new(primitive: Primitive, color: Vec3) -> Self {
        Self {
            primitive,
            color,
            lit: true,
        }
    }

    #[allow(dead_code)]
    pub fn unlit(mut self) -> Self {
        self.lit = false;
        self
    }
}

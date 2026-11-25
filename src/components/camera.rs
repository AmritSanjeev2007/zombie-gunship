use glam::Mat4;

/// A perspective camera. Its view comes from the entity's [`Transform`];
/// the camera system picks the first `active` one each frame.
///
/// [`Transform`]: super::Transform
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// Vertical field of view in degrees.
    pub fov_y_degrees: f32,
    pub near: f32,
    pub far: f32,
    pub active: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov_y_degrees: 60.0,
            near: 0.1,
            far: 1000.0,
            active: true,
        }
    }
}

impl Camera {
    pub fn projection(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov_y_degrees.to_radians(), aspect, self.near, self.far)
    }
}

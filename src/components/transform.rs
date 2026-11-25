use glam::{Mat4, Quat, Vec3};

/// Position, orientation and scale of an entity in world space.
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Default::default()
        }
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Model matrix (local → world).
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Points the entity's -Z axis at `target`, with +Y as up.
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let view = Mat4::look_at_rh(self.translation, target, up);
        self.rotation = Quat::from_mat4(&view.inverse());
    }

    #[allow(dead_code)] // for upcoming gunship/weapon systems
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_at_faces_target() {
        let mut t = Transform::from_translation(Vec3::new(10.0, 5.0, 0.0));
        t.look_at(Vec3::ZERO, Vec3::Y);
        let expected = (Vec3::ZERO - t.translation).normalize();
        assert!(t.forward().abs_diff_eq(expected, 1e-5));
    }
}

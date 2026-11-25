use glam::Vec3;

/// Makes an entity circle a target point, like a gunship banking around a
/// battlefield. Angles are in radians.
///
/// Drives the entity's [`Transform`](super::Transform); pair it with a
/// [`Camera`](super::Camera) for an orbiting camera.
#[derive(Clone, Copy, Debug)]
pub struct OrbitController {
    pub target: Vec3,
    pub radius: f32,
    /// Angle around the Y axis.
    pub azimuth: f32,
    /// Angle above the ground plane.
    pub elevation: f32,
    /// Radians per second added to `azimuth` while `auto_rotate` is on.
    pub angular_speed: f32,
    pub auto_rotate: bool,
    pub min_radius: f32,
    pub max_radius: f32,
    pub min_elevation: f32,
    pub max_elevation: f32,
    /// Radians per pixel of mouse drag.
    pub drag_sensitivity: f32,
    /// Fraction of the radius changed per scroll line.
    pub zoom_sensitivity: f32,
}

impl Default for OrbitController {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 30.0,
            azimuth: 0.0,
            elevation: 30f32.to_radians(),
            angular_speed: 15f32.to_radians(),
            auto_rotate: true,
            min_radius: 5.0,
            max_radius: 200.0,
            min_elevation: -10f32.to_radians(),
            max_elevation: 85f32.to_radians(),
            drag_sensitivity: 0.005,
            zoom_sensitivity: 0.1,
        }
    }
}

impl OrbitController {
    /// World-space position on the orbit for the current angles.
    pub fn position(&self) -> Vec3 {
        let (sin_az, cos_az) = self.azimuth.sin_cos();
        let (sin_el, cos_el) = self.elevation.sin_cos();
        self.target + self.radius * Vec3::new(cos_el * sin_az, sin_el, cos_el * cos_az)
    }

    /// Keeps the angles and radius inside their limits.
    pub fn clamp(&mut self) {
        self.radius = self.radius.clamp(self.min_radius, self.max_radius);
        self.elevation = self.elevation.clamp(self.min_elevation, self.max_elevation);
        self.azimuth = self.azimuth.rem_euclid(std::f32::consts::TAU);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_is_at_radius_from_target() {
        let orbit = OrbitController {
            target: Vec3::new(1.0, 2.0, 3.0),
            radius: 12.0,
            azimuth: 1.1,
            elevation: 0.4,
            ..Default::default()
        };
        let distance = orbit.position().distance(orbit.target);
        assert!((distance - 12.0).abs() < 1e-4);
    }
}

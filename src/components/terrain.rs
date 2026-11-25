use glam::Vec3;

/// Draws the current map's heightfield (from
/// [`Resources::map`](crate::engine::Resources::map)).
#[derive(Clone, Copy, Debug)]
pub struct Terrain {
    pub low_color: Vec3,
    pub high_color: Vec3,
    pub rock_color: Vec3,
    /// Draw zombie routes, path-tree links and the objective ring on top.
    pub show_paths: bool,
}

impl Default for Terrain {
    fn default() -> Self {
        Self {
            low_color: Vec3::new(0.22, 0.13, 0.10),
            high_color: Vec3::new(0.62, 0.45, 0.32),
            rock_color: Vec3::new(0.35, 0.31, 0.29),
            show_paths: false,
        }
    }
}

/// Tags entities that belong to the current map (objective and spawn
/// markers), so they can be cleared when the map is regenerated.
#[derive(Clone, Copy, Debug, Default)]
pub struct MapMarker;

//! Seeded Perlin noise, fractal Brownian motion and the [`Heightfield`] grid.

use glam::{Vec2, Vec3};
use rand::Rng;
use rand::seq::SliceRandom;

/// Settings for the fractal noise that shapes the base terrain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoiseConfig {
    /// Number of noise layers summed together.
    pub octaves: u32,
    /// Features per world unit of the first octave.
    pub frequency: f32,
    /// Frequency multiplier between octaves.
    pub lacunarity: f32,
    /// Amplitude multiplier between octaves.
    pub persistence: f32,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            octaves: 4,
            frequency: 1.0 / 45.0,
            lacunarity: 2.0,
            persistence: 0.5,
        }
    }
}

/// Classic 2D gradient (Perlin) noise with a seeded permutation table.
#[derive(Clone)]
pub struct Perlin {
    perm: [u8; 512],
}

impl Perlin {
    pub fn new(rng: &mut impl Rng) -> Self {
        let mut table: Vec<u8> = (0..=255).collect();
        table.shuffle(rng);
        let mut perm = [0u8; 512];
        for i in 0..512 {
            perm[i] = table[i & 255];
        }
        Self { perm }
    }

    fn hash(&self, x: i32, y: i32) -> u8 {
        let xi = (x & 255) as usize;
        let yi = (y & 255) as usize;
        self.perm[self.perm[xi] as usize + yi]
    }

    fn grad(hash: u8, x: f32, y: f32) -> f32 {
        // Eight evenly spread gradient directions.
        match hash & 7 {
            0 => x + y,
            1 => x - y,
            2 => -x + y,
            3 => -x - y,
            4 => x,
            5 => -x,
            6 => y,
            _ => -y,
        }
    }

    /// Noise in roughly `[-1, 1]`, zero at integer lattice points.
    pub fn sample(&self, p: Vec2) -> f32 {
        let x0 = p.x.floor();
        let y0 = p.y.floor();
        let (fx, fy) = (p.x - x0, p.y - y0);
        let (xi, yi) = (x0 as i32, y0 as i32);
        let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        let (u, v) = (fade(fx), fade(fy));

        let n00 = Self::grad(self.hash(xi, yi), fx, fy);
        let n10 = Self::grad(self.hash(xi + 1, yi), fx - 1.0, fy);
        let n01 = Self::grad(self.hash(xi, yi + 1), fx, fy - 1.0);
        let n11 = Self::grad(self.hash(xi + 1, yi + 1), fx - 1.0, fy - 1.0);

        let a = n00 + u * (n10 - n00);
        let b = n01 + u * (n11 - n01);
        // The diagonal gradients reach ±1.0 at most along the diagonal.
        (a + v * (b - a)) * 0.9
    }

    /// Fractal sum of `octaves` layers, normalised to roughly `[-1, 1]`.
    pub fn fbm(&self, p: Vec2, cfg: &NoiseConfig) -> f32 {
        let mut sum = 0.0;
        let mut amplitude = 1.0;
        let mut total = 0.0;
        let mut frequency = cfg.frequency;
        for octave in 0..cfg.octaves.max(1) {
            // Offset each octave so lattice zeros don't line up.
            let offset = Vec2::new(17.3, -41.9) * octave as f32;
            sum += amplitude * self.sample(p * frequency + offset);
            total += amplitude;
            amplitude *= cfg.persistence;
            frequency *= cfg.lacunarity;
        }
        sum / total
    }
}

/// A regular grid of heights covering the map rectangle.
///
/// Sample `(i, j)` sits at world `x = -width/2 + i·dx`, `z = -depth/2 + j·dz`,
/// so the outermost samples lie exactly on the map edges.
#[derive(Clone, Debug, PartialEq)]
pub struct Heightfield {
    /// Samples along X.
    pub cols: usize,
    /// Samples along Z.
    pub rows: usize,
    pub size: Vec2,
    pub data: Vec<f32>,
}

impl Heightfield {
    pub fn new(cols: usize, rows: usize, size: Vec2) -> Self {
        let cols = cols.max(2);
        let rows = rows.max(2);
        Self {
            cols,
            rows,
            size,
            data: vec![0.0; cols * rows],
        }
    }

    /// Distance between neighbouring samples along X and Z.
    pub fn spacing(&self) -> Vec2 {
        Vec2::new(
            self.size.x / (self.cols - 1) as f32,
            self.size.y / (self.rows - 1) as f32,
        )
    }

    pub fn index(&self, i: usize, j: usize) -> usize {
        j * self.cols + i
    }

    pub fn get(&self, i: usize, j: usize) -> f32 {
        self.data[self.index(i, j)]
    }

    /// World XZ position of sample `(i, j)`.
    pub fn position(&self, i: usize, j: usize) -> Vec2 {
        -self.size * 0.5 + Vec2::new(i as f32, j as f32) * self.spacing()
    }

    /// Fractional grid coordinates of a world XZ position.
    pub fn grid_coords(&self, p: Vec2) -> Vec2 {
        (p + self.size * 0.5) / self.spacing()
    }

    /// Ranges of sample indices whose positions lie inside the box
    /// `min..=max` (clamped to the grid). `None` if the box is off the map.
    pub fn index_range(
        &self,
        min: Vec2,
        max: Vec2,
    ) -> Option<(
        std::ops::RangeInclusive<usize>,
        std::ops::RangeInclusive<usize>,
    )> {
        let lo = self.grid_coords(min).ceil();
        let hi = self.grid_coords(max).floor();
        let (cols, rows) = ((self.cols - 1) as f32, (self.rows - 1) as f32);
        if hi.x < 0.0 || hi.y < 0.0 || lo.x > cols || lo.y > rows {
            return None;
        }
        let i0 = lo.x.max(0.0) as usize;
        let j0 = lo.y.max(0.0) as usize;
        let i1 = hi.x.min(cols) as usize;
        let j1 = hi.y.min(rows) as usize;
        Some((i0..=i1, j0..=j1))
    }

    /// Bilinearly interpolated height at a world XZ position (clamped to the
    /// map).
    pub fn sample(&self, p: Vec2) -> f32 {
        let g = self.grid_coords(p);
        let gx = g.x.clamp(0.0, (self.cols - 1) as f32);
        let gy = g.y.clamp(0.0, (self.rows - 1) as f32);
        let i0 = (gx.floor() as usize).min(self.cols - 2);
        let j0 = (gy.floor() as usize).min(self.rows - 2);
        let (tx, ty) = (gx - i0 as f32, gy - j0 as f32);
        let a = self.get(i0, j0) + tx * (self.get(i0 + 1, j0) - self.get(i0, j0));
        let b = self.get(i0, j0 + 1) + tx * (self.get(i0 + 1, j0 + 1) - self.get(i0, j0 + 1));
        a + ty * (b - a)
    }

    /// Surface normal at sample `(i, j)` from central differences.
    pub fn normal(&self, i: usize, j: usize) -> Vec3 {
        let s = self.spacing();
        let (il, ir) = (i.saturating_sub(1), (i + 1).min(self.cols - 1));
        let (jd, ju) = (j.saturating_sub(1), (j + 1).min(self.rows - 1));
        let dhdx = (self.get(ir, j) - self.get(il, j)) / ((ir - il) as f32 * s.x);
        let dhdz = (self.get(i, ju) - self.get(i, jd)) / ((ju - jd) as f32 * s.y);
        Vec3::new(-dhdx, 1.0, -dhdz).normalize()
    }

    /// Builds a heightfield by evaluating `f` at every sample position.
    pub fn from_fn(cols: usize, rows: usize, size: Vec2, f: impl Fn(Vec2) -> f32) -> Self {
        let mut field = Self::new(cols, rows, size);
        for j in 0..field.rows {
            for i in 0..field.cols {
                let idx = field.index(i, j);
                field.data[idx] = f(field.position(i, j));
            }
        }
        field
    }
}

/// The uncarved terrain: the full fBm heights, plus a smooth layer (first
/// octave only) that carved floors blend toward.
pub struct BaseTerrain {
    pub detail: Heightfield,
    pub smooth: Heightfield,
}

/// Generates the base terrain. Heights span `[floor, max_height]`, where the
/// lower `floor` share leaves room for craters and valleys to dig into.
pub fn base_terrain(
    noise: &Perlin,
    cfg: &NoiseConfig,
    cols: usize,
    rows: usize,
    size: Vec2,
    max_height: f32,
) -> BaseTerrain {
    let floor = 0.3 * max_height;
    // fBm rarely strays far from zero, so stretch it before mapping.
    let to_height = |n: f32| floor + (max_height - floor) * (n * 0.8 + 0.5).clamp(0.0, 1.0);
    let smooth_cfg = NoiseConfig { octaves: 1, ..*cfg };
    BaseTerrain {
        detail: Heightfield::from_fn(cols, rows, size, |p| to_height(noise.fbm(p, cfg))),
        smooth: Heightfield::from_fn(cols, rows, size, |p| to_height(noise.fbm(p, &smooth_cfg))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn perlin_stays_in_range_and_varies() {
        let noise = Perlin::new(&mut ChaCha8Rng::seed_from_u64(1));
        let cfg = NoiseConfig::default();
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for i in 0..2000 {
            let p = Vec2::new(i as f32 * 0.731, i as f32 * 0.377);
            let v = noise.fbm(p, &cfg);
            lo = lo.min(v);
            hi = hi.max(v);
        }
        assert!(lo >= -1.0 && hi <= 1.0, "{lo}..{hi}");
        assert!(hi - lo > 0.3, "noise is nearly flat: {lo}..{hi}");
    }

    #[test]
    fn sample_matches_grid_points_and_edges() {
        let f = Heightfield::from_fn(5, 3, Vec2::new(8.0, 4.0), |p| p.x + 10.0 * p.y);
        assert_eq!(f.position(0, 0), Vec2::new(-4.0, -2.0));
        assert_eq!(f.position(4, 2), Vec2::new(4.0, 2.0));
        let p = Vec2::new(1.3, -0.7);
        assert!((f.sample(p) - (p.x + 10.0 * p.y)).abs() < 1e-4);
    }
}

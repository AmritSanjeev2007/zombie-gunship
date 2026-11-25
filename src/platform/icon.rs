//! Rasterises `assets/favicon.svg` into a window icon.

use anyhow::{Context as _, Result, anyhow};
use resvg::{tiny_skia, usvg};

/// The skull icon, embedded so it works no matter where the binary runs.
const FAVICON_SVG: &[u8] = include_bytes!("../../assets/favicon.svg");

/// Renders the favicon to straight (non-premultiplied) RGBA pixels.
pub fn favicon_rgba(size: u32) -> Result<Vec<u8>> {
    let tree = usvg::Tree::from_data(FAVICON_SVG, &usvg::Options::default())
        .context("parsing favicon.svg")?;
    let mut pixmap =
        tiny_skia::Pixmap::new(size, size).ok_or_else(|| anyhow!("invalid icon size"))?;
    let svg_size = tree.size();
    let transform = tiny_skia::Transform::from_scale(
        size as f32 / svg_size.width(),
        size as f32 / svg_size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Ok(pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect())
}

pub fn window_icon() -> Result<winit::window::Icon> {
    const SIZE: u32 = 64;
    let rgba = favicon_rgba(SIZE)?;
    winit::window::Icon::from_rgba(rgba, SIZE, SIZE).map_err(|e| anyhow!(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favicon_renders_something() {
        let rgba = favicon_rgba(32).unwrap();
        assert_eq!(rgba.len(), 32 * 32 * 4);
        assert!(
            rgba.chunks(4).any(|p| p[3] > 0),
            "icon is fully transparent"
        );
    }
}

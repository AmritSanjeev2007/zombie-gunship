use std::path::Path;

use anyhow::{Context as _, Result, anyhow};
use glow::HasContext;

#[allow(dead_code)]
/// A 2D RGBA8 texture loaded from an image file.
pub struct Texture {
    texture: glow::Texture,
    pub width: u32,
    pub height: u32,
}

/// How the texture is sampled outside [0, 1] horizontally.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum Wrap {
    Repeat,
    Clamp,
}

impl Texture {
    /// Loads an image. Row 0 of the image ends up at `t = 0`, so a shader
    /// sampling with `t = 0` at the top of the picture needs no flip.
    /// + Passing `flipv = true` flips the image vertically when generating the texture.
    /// + Passing `fliph = true` flips the image horizontally when generating the texure.
    pub fn from_file(gl: &glow::Context, path: &Path, wrap_s: Wrap, flipv: bool, fliph: bool) -> Result<Self> {
        let mut image_handle = image::open(path)
            .with_context(|| format!("loading texture {}", path.display()))?;

        if flipv { image_handle = image_handle.flipv(); };
        if fliph { image_handle = image_handle.fliph(); };

        let image = image_handle.into_rgba8();
        let (width, height) = image.dimensions();

        unsafe {
            let texture = gl.create_texture().map_err(|e| anyhow!(e))?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(image.as_raw())),
            );
            let wrap = match wrap_s {
                Wrap::Repeat => glow::REPEAT,
                Wrap::Clamp => glow::CLAMP_TO_EDGE,
            };
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap as i32);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);

            Ok(Self {
                texture,
                width,
                height,
            })
        }
    }

    pub fn bind(&self, gl: &glow::Context, unit: u32) {
        unsafe {
            gl.active_texture(glow::TEXTURE0 + unit);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe { gl.delete_texture(self.texture) }
    }
}

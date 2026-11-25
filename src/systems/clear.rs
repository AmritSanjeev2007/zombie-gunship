use glow::HasContext;

use crate::engine::{FrameContext, System};

/// Sets the viewport and clears the screen. Register it before any renderer.
pub struct ClearSystem {
    pub color: [f32; 3],
}

impl System for ClearSystem {
    fn name(&self) -> &'static str {
        "Clear"
    }

    fn render(&mut self, ctx: &mut FrameContext) {
        let gl = ctx.gl;
        let vp = ctx.resources.viewport;
        let [r, g, b] = self.color;
        unsafe {
            // The UI pass leaves scissor/blend enabled; reset before clearing.
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::BLEND);
            gl.viewport(0, 0, vp.width as i32, vp.height as i32);
            gl.clear_color(r, g, b, 1.0);
            gl.depth_mask(true);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }
    }
}

use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow};
use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, GlProfile, PossiblyCurrentContext, Version,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::{GlSurface, NotCurrentGlContext};
use glutin::surface::{Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow as _};
use raw_window_handle::HasWindowHandle;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

/// A window with a current OpenGL 3.3 core context.
pub struct GlWindow {
    pub window: Window,
    pub gl: Arc<glow::Context>,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
}

impl GlWindow {
    pub fn new(event_loop: &ActiveEventLoop, attributes: WindowAttributes) -> Result<Self> {
        // Only require a depth buffer; `pick_config` takes the best MSAA
        // level on offer, so machines without MSAA still get a window.
        let template = ConfigTemplateBuilder::new().with_depth_size(24);

        let (window, config) = DisplayBuilder::new()
            .with_window_attributes(Some(attributes))
            .build(event_loop, template, pick_config)
            .map_err(|e| anyhow!("creating window: {e}"))?;
        let window = window.context("no window was created")?;

        let raw_handle = window.window_handle().ok().map(|h| h.as_raw());
        let display = config.display();
        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .with_profile(GlProfile::Core)
            .build(raw_handle);

        let not_current = unsafe { display.create_context(&config, &context_attributes) }
            .context("creating an OpenGL 3.3 core context (is your GPU driver up to date?)")?;

        let surface_attributes = window
            .build_surface_attributes(Default::default())
            .context("building surface attributes")?;
        let surface = unsafe { display.create_window_surface(&config, &surface_attributes) }
            .context("creating window surface")?;
        let context = not_current
            .make_current(&surface)
            .context("making context current")?;

        // VSync; not fatal if the driver refuses.
        let _ = surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::MIN));

        let gl =
            unsafe { glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s)) };

        Ok(Self {
            window,
            gl: Arc::new(gl),
            surface,
            context,
        })
    }

    pub fn resize(&self, width: u32, height: u32) {
        if let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
            self.surface.resize(&self.context, w, h);
        }
    }

    pub fn swap_buffers(&self) -> Result<()> {
        self.surface
            .swap_buffers(&self.context)
            .context("swapping buffers")
    }
}

/// Prefers the config with the most MSAA samples.
fn pick_config(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    configs
        .reduce(|best, c| {
            if c.num_samples() > best.num_samples() {
                c
            } else {
                best
            }
        })
        .expect("no OpenGL configs available")
}

//! Window lifecycle and the main loop. Knows nothing about cameras or skies;
//! it just pumps events into the [`Engine`] and presents frames.

use anyhow::Result;
use egui_glow::EguiGlow;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::engine::{Engine, Viewport};
use crate::platform::{GlWindow, icon};
use crate::scene;

pub fn run() -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    match app.error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

#[derive(Default)]
struct App {
    running: Option<Running>,
    error: Option<anyhow::Error>,
}

/// Everything that exists once the window is up.
struct Running {
    gl_window: GlWindow,
    egui: EguiGlow,
    engine: Engine,
}

impl Running {
    fn new(event_loop: &ActiveEventLoop) -> Result<Self> {
        let mut attributes = Window::default_attributes()
            .with_title("Zombie Gunship")
            .with_inner_size(LogicalSize::new(1280.0, 720.0));
        match icon::window_icon() {
            Ok(icon) => attributes = attributes.with_window_icon(Some(icon)),
            Err(err) => eprintln!("window icon: {err:#}"),
        }

        let gl_window = GlWindow::new(event_loop, attributes)?;
        let egui = EguiGlow::new(
            event_loop,
            gl_window.gl.clone(),
            None,
            Some(gl_window.window.scale_factor() as f32),
            true,
        );

        let size = gl_window.window.inner_size();
        let mut engine = Engine::new(Viewport {
            width: size.width,
            height: size.height,
        });
        scene::build(&mut engine);

        Ok(Self {
            gl_window,
            egui,
            engine,
        })
    }

    fn frame(&mut self) -> Result<()> {
        let Self {
            gl_window,
            egui,
            engine,
        } = self;

        engine.update();
        engine.render(&gl_window.gl);

        egui.run(&gl_window.window, |ui| engine.ui(ui.ctx()));
        egui.paint(&gl_window.window);

        gl_window.swap_buffers()
    }

    fn destroy(mut self) {
        self.engine.destroy(&self.gl_window.gl);
        self.egui.destroy();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_some() {
            return;
        }
        match Running::new(event_loop) {
            Ok(running) => self.running = Some(running),
            Err(err) => {
                self.error = Some(err);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(running) = &mut self.running else {
            return;
        };

        let response = running
            .egui
            .on_window_event(&running.gl_window.window, &event);
        running
            .engine
            .resources
            .input
            .handle_event(&event, response.consumed);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                running.gl_window.resize(size.width, size.height);
                running.engine.resources.viewport = Viewport {
                    width: size.width,
                    height: size.height,
                };
            }
            WindowEvent::RedrawRequested => {
                if let Err(err) = running.frame() {
                    self.error = Some(err);
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(running) = &self.running {
            running.gl_window.window.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(running) = self.running.take() {
            running.destroy();
        }
    }
}

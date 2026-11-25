use std::collections::HashSet;

use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Mouse and keyboard state, fed from window events.
///
/// Per-frame deltas (`mouse_delta`, `scroll_delta`) are reset after every
/// update, so systems should read them during `update`.
#[derive(Default)]
pub struct Input {
    keys_down: HashSet<KeyCode>,
    buttons_down: HashSet<MouseButton>,
    last_cursor: Option<(f32, f32)>,
    /// Cursor movement this frame, in physical pixels.
    pub mouse_delta: (f32, f32),
    /// Scroll this frame, in "lines" (positive = away from the user).
    pub scroll_delta: f32,
}

impl Input {
    pub fn key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn button_down(&self, button: MouseButton) -> bool {
        self.buttons_down.contains(&button)
    }

    /// Feeds a window event. `ui_captured` should be true when the UI already
    /// handled the event, so that dragging a slider doesn't spin the camera.
    pub fn handle_event(&mut self, event: &WindowEvent, ui_captured: bool) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed if !ui_captured => {
                            self.keys_down.insert(code);
                        }
                        ElementState::Released => {
                            self.keys_down.remove(&code);
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed if !ui_captured => {
                    self.buttons_down.insert(*button);
                }
                ElementState::Released => {
                    self.buttons_down.remove(button);
                }
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                let pos = (position.x as f32, position.y as f32);
                if let Some(last) = self.last_cursor {
                    self.mouse_delta.0 += pos.0 - last.0;
                    self.mouse_delta.1 += pos.1 - last.1;
                }
                self.last_cursor = Some(pos);
            }
            WindowEvent::CursorLeft { .. } => self.last_cursor = None,
            WindowEvent::MouseWheel { delta, .. } if !ui_captured => {
                self.scroll_delta += match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
            }
            WindowEvent::Focused(false) => {
                self.keys_down.clear();
                self.buttons_down.clear();
            }
            _ => {}
        }
    }

    pub(super) fn end_frame(&mut self) {
        self.mouse_delta = (0.0, 0.0);
        self.scroll_delta = 0.0;
    }
}

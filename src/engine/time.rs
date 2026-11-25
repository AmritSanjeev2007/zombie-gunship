use std::time::Instant;

/// Frame timing.
pub struct Time {
    start: Instant,
    last: Instant,
    /// Seconds since the previous frame, clamped to avoid huge jumps after a stall.
    pub delta: f32,
    /// Seconds since the engine started.
    pub elapsed: f32,
    /// Smoothed frames per second, for display.
    pub fps: f32,
}

impl Time {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last: now,
            delta: 0.0,
            elapsed: 0.0,
            fps: 0.0,
        }
    }

    pub(super) fn tick(&mut self) {
        let now = Instant::now();
        self.delta = (now - self.last).as_secs_f32().min(0.1);
        self.elapsed = (now - self.start).as_secs_f32();
        self.last = now;
        if self.delta > 0.0 {
            let instant_fps = 1.0 / self.delta;
            self.fps = if self.fps == 0.0 {
                instant_fps
            } else {
                self.fps * 0.95 + instant_fps * 0.05
            };
        }
    }
}

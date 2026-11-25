use crate::components::{Camera, Name, OrbitController, Skybox};
use crate::engine::{System, UiContext};

/// The developer overlay: frame stats, live-tweakable components and the
/// list of entities and systems. Toggle with F1.
pub struct DebugUiSystem {
    pub open: bool,
    f1_was_down: bool,
}

impl Default for DebugUiSystem {
    fn default() -> Self {
        Self {
            open: true,
            f1_was_down: false,
        }
    }
}

impl System for DebugUiSystem {
    fn name(&self) -> &'static str {
        "Debug UI"
    }

    fn ui(&mut self, ctx: &mut UiContext) {
        let f1 = ctx.resources.input.key_down(winit::keyboard::KeyCode::F1);
        if f1 && !self.f1_was_down {
            self.open = !self.open;
        }
        self.f1_was_down = f1;

        let mut open = self.open;
        egui::Window::new("Zombie Gunship")
            .open(&mut open)
            .default_pos([12.0, 12.0])
            .default_width(280.0)
            .resizable(false)
            .show(ctx.egui, |ui| {
                let time = &ctx.resources.time;
                ui.label(format!(
                    "{:.0} FPS ({:.2} ms)   {}×{}",
                    time.fps,
                    1000.0 / time.fps.max(1.0),
                    ctx.resources.viewport.width,
                    ctx.resources.viewport.height
                ));
                ui.small("Left-drag to orbit, scroll to zoom, F1 to hide.");
                ui.separator();

                for (name, orbit, camera) in
                    ctx.world
                        .query_mut::<(Option<&Name>, &mut OrbitController, Option<&mut Camera>)>()
                {
                    let title = name.map_or("Orbit", |n| n.0.as_str());
                    egui::CollapsingHeader::new(title)
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.checkbox(&mut orbit.auto_rotate, "Auto-rotate");
                            let mut speed = orbit.angular_speed.to_degrees();
                            ui.add(egui::Slider::new(&mut speed, -90.0..=90.0).text("speed °/s"));
                            orbit.angular_speed = speed.to_radians();

                            let mut az = orbit.azimuth.to_degrees();
                            ui.add(egui::Slider::new(&mut az, 0.0..=360.0).text("azimuth °"));
                            orbit.azimuth = az.to_radians();

                            let mut el = orbit.elevation.to_degrees();
                            let (lo, hi) = (
                                orbit.min_elevation.to_degrees(),
                                orbit.max_elevation.to_degrees(),
                            );
                            ui.add(egui::Slider::new(&mut el, lo..=hi).text("elevation °"));
                            orbit.elevation = el.to_radians();

                            let range = orbit.min_radius..=orbit.max_radius;
                            ui.add(
                                egui::Slider::new(&mut orbit.radius, range)
                                    .logarithmic(true)
                                    .text("radius"),
                            );

                            if let Some(camera) = camera {
                                ui.add(
                                    egui::Slider::new(&mut camera.fov_y_degrees, 20.0..=110.0)
                                        .text("FOV °"),
                                );
                            }
                        });
                }

                for (name, skybox) in ctx.world.query_mut::<(Option<&Name>, &mut Skybox)>() {
                    let title = name.map_or("Skybox", |n| n.0.as_str());
                    egui::CollapsingHeader::new(title).show(ui, |ui| {
                        let mut tint = skybox.tint.to_array();
                        ui.horizontal(|ui| {
                            ui.label("tint");
                            ui.color_edit_button_rgb(&mut tint);
                        });
                        skybox.tint = tint.into();
                        let mut rot = skybox.rotation.to_degrees();
                        ui.add(egui::Slider::new(&mut rot, 0.0..=360.0).text("rotation °"));
                        skybox.rotation = rot.to_radians();
                        ui.small(skybox.texture.display().to_string());
                    });
                }

                egui::CollapsingHeader::new(format!("Entities ({})", ctx.world.len())).show(
                    ui,
                    |ui| {
                        for entity in ctx.world.iter() {
                            let label = entity
                                .get::<&Name>()
                                .map_or_else(|| format!("{:?}", entity.entity()), |n| n.0.clone());
                            ui.label(label);
                        }
                    },
                );

                egui::CollapsingHeader::new(format!("Systems ({})", ctx.system_names.len())).show(
                    ui,
                    |ui| {
                        for (i, name) in ctx.system_names.iter().enumerate() {
                            ui.label(format!("{}. {name}", i + 1));
                        }
                    },
                );
            });
        self.open = open;
    }
}

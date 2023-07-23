use bevy::prelude::*;

use bevy_egui::EguiContexts;

#[derive(Resource)]
pub struct GlobalSettings {
    pub show_infeasible: bool,
    pub enable_time_animation: bool,
    pub time_animation_speed: f32,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            show_infeasible: false,
            enable_time_animation: false,
            time_animation_speed: 5.0,
        }
    }
}

#[derive(Default, Resource)]
pub struct CurrentTimeStep {
    pub time_step: i32,
    pub dynamic_time_step: f32,
}

pub fn animate_time(
    time: Res<Time>,
    mut cts: ResMut<CurrentTimeStep>,
    settings: Res<GlobalSettings>
) {
    if !settings.enable_time_animation { return; }

    cts.dynamic_time_step += time.delta_seconds() * settings.time_animation_speed;

    if cts.dynamic_time_step > 200.0 {
        cts.dynamic_time_step -= 200.0;
    }
}

pub fn side_panel(mut contexts: EguiContexts, mut cts: ResMut<CurrentTimeStep>, mut settings: ResMut<GlobalSettings>, cr: Res<crate::CommonRoad>) {
    let ctx = contexts.ctx_mut();

    let panel_id = egui::Id::new("side panel left");
    egui::SidePanel::left(panel_id)
        .resizable(false)
        .exact_width(450.0)
        .show(ctx, |ui| {
            ui.heading("Scenario Information");
            ui.label(format!("{:#?}", cr.information));

            ui.heading("Display Settings");
            ui.checkbox(&mut settings.show_infeasible, "Show infeasible trajectories");

            ui.heading("Time Control");
            ui.checkbox(&mut settings.enable_time_animation, "Enable time progression");

            ui.style_mut().spacing.slider_width = 300.0;
            ui.add(
                egui::Slider::new(&mut cts.time_step, 0..=40)
                    .text("time step")
                    .step_by(1.0)
                    .clamp_to_range(true),
            );

            ui.add(
                egui::Slider::new(&mut cts.dynamic_time_step, 0.0..=200.0)
                    .smart_aim(false)
                    .text("d time step")
                    .clamp_to_range(true),
            );
            ui.add(
                egui::Slider::new(&mut settings.time_animation_speed, 0.1..=100.0)
                    .logarithmic(true)
                    .text("speed")
                    .clamp_to_range(true),
            );
        });
}

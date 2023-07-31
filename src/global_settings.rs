use bevy::prelude::*;

use bevy_egui::EguiContexts;


pub struct GlobalSettingsPlugin;

impl Plugin for GlobalSettingsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<GlobalSettings>()
            .init_resource::<TimeStep>()
            .add_systems(Update, side_panel)
            .add_systems(Update, animate_time);
    }
}

#[derive(Resource, Clone, PartialEq)]
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

#[derive(Resource)]
pub struct CurrentTimeStep {
    pub dynamic_time_step: f32,
    pub prediction_range: std::ops::RangeInclusive<f32>,
}

impl CurrentTimeStep {
    pub(crate) fn fixed_time_step(&self) -> i32 {
        self.dynamic_time_step.round() as i32
    }
}

#[derive(Default, Resource)]
pub struct TimeStep {
    pub time_step: i32,
}

pub fn animate_time(
    time: Res<Time>,
    mut cts: ResMut<CurrentTimeStep>,
    settings: Res<GlobalSettings>,
) {
    if !settings.enable_time_animation {
        return;
    }

    cts.dynamic_time_step += time.delta_seconds() * settings.time_animation_speed;

    let ts_end = *cts.prediction_range.end();
    while cts.dynamic_time_step > ts_end {
        cts.dynamic_time_step -= ts_end;
    }

    if cts.dynamic_time_step < 0.0 {
        bevy::log::warn!("dynamic_time_step was below zero, resetting");
        cts.dynamic_time_step = 0.0;
    }
}

pub fn side_panel(
    mut contexts: EguiContexts,
    mut cts: ResMut<CurrentTimeStep>,
    mut ts: ResMut<TimeStep>,
    mut settings: ResMut<GlobalSettings>,
    cr: Res<crate::CommonRoad>,
) {
    let ctx = contexts.ctx_mut();

    let mut new_settings = settings.to_owned();

    let panel_id = egui::Id::new("side panel left");
    egui::SidePanel::left(panel_id)
        .resizable(false)
        .exact_width(450.0)
        .show(ctx, |ui| {
            ui.heading("Scenario Information");
            ui.label(format!("{:#?}", cr.information));

            ui.heading("Display Settings");
            ui.checkbox(
                &mut new_settings.show_infeasible,
                "Show infeasible trajectories",
            );

            ui.heading("Time Control");
            ui.checkbox(
                &mut new_settings.enable_time_animation,
                "Enable time progression",
            );

            ui.style_mut().spacing.slider_width = 300.0;
            let range = cts.prediction_range.clone();
            ui.add(
                egui::Slider::new(&mut cts.dynamic_time_step, range)
                    .smart_aim(false)
                    .text("time step")
                    .clamp_to_range(true)
                    .trailing_fill(true),
            );
            ui.add(
                egui::Slider::new(&mut new_settings.time_animation_speed, 0.1..=100.0)
                    .logarithmic(true)
                    .text("speed")
                    .clamp_to_range(true),
            );
        });

    settings.set_if_neq(new_settings);

    let new_ts = cts.dynamic_time_step.round() as i32;
    if new_ts != ts.time_step {
        ts.time_step = new_ts;
    }
}

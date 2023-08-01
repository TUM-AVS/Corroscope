use bevy::prelude::*;

use bevy_egui::EguiContexts;

pub struct GlobalSettingsPlugin;

impl Plugin for GlobalSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobalSettings>()
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
    fn fixed_time_step(&self) -> i32 {
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

    egui::Window::new("Display Settings")
        .show(ctx, |ui| {
            ui.checkbox(
                &mut new_settings.show_infeasible,
                "Show infeasible trajectories",
            );
        });

    // let panel_id = egui::Id::new("side panel left");
    egui::Window::new("\u{23F1} Time Control")
        // .resizable(false)
        // .exact_width(450.0)
        // .auto_sized()
        .default_width(310.0)
        .resizable(false)
        .show(ctx, |ui| {
            // ui.heading("Scenario Information");
            // ui.label(format!("{:#?}", cr.information));

            //ctx.settings_ui(ui);

            ui.set_max_width(310.0);
            ui.allocate_ui_with_layout(
                egui::Vec2::new(300.0, 60.0),
                egui::Layout::left_to_right(egui::Align::Min)
                    .with_main_align(egui::Align::Center)
                    .with_main_justify(false),
                |ui| {
                let symbol_text = |c: char| {
                    egui::RichText::new(c).size(24.0).monospace()
                };
                let symbol_button = |c: char| {
                    egui::Button::new(symbol_text(c))
                        .wrap(false)
                        .min_size(egui::Vec2::splat(50.0))
                        .sense(egui::Sense::click_and_drag())
                };

                if ui.add(symbol_button('\u{23EE}')).clicked() {
                    cts.dynamic_time_step = *cts.prediction_range.start();
                }

                let resp = ui.add(symbol_button('\u{23EA}'));
                if resp.clicked() {
                    cts.dynamic_time_step = cts.dynamic_time_step.round() - 1.0;
                }
                if resp.dragged() {
                    cts.dynamic_time_step = cts.dynamic_time_step - 0.2;
                }

                let current_symbol =
                    if new_settings.enable_time_animation { '\u{23F8}' } else { '\u{23F5}' };

                if ui.add(symbol_button(current_symbol)).clicked() {
                    new_settings.enable_time_animation = !new_settings.enable_time_animation;
                }

                let resp = ui.add(symbol_button('\u{23E9}'));
                if resp.clicked() {
                    cts.dynamic_time_step = cts.dynamic_time_step.round() + 1.0;
                }
                if resp.dragged() {
                    cts.dynamic_time_step = cts.dynamic_time_step + 0.2;
                }

                if ui.add(symbol_button('\u{23ED}')).clicked() {
                    cts.dynamic_time_step = *cts.prediction_range.end();
                }
            });

            ui.style_mut().spacing.slider_width = 300.0;
            let range = cts.prediction_range.clone();
            ui.add(
                egui::Slider::new(&mut cts.dynamic_time_step, range)
                    .text("time step")
                    .clamp_to_range(true)
                    .max_decimals(0)
                    .trailing_fill(true),
            );
            let mut realtime_speed = new_settings.time_animation_speed * cr.information.time_step_size as f32;
            ui.add(
                egui::Slider::new(&mut realtime_speed, 0.1..=10.0)
                    .logarithmic(true)
                    .text("speed")
                    .suffix("\u{00D7}")
                    .clamp_to_range(true),
            ).on_hover_text("Speed in actual time per seconds (1\u{00D7} is realtime)");
            new_settings.time_animation_speed = realtime_speed / cr.information.time_step_size as f32;
        });

    settings.set_if_neq(new_settings);

    let new_ts = cts.fixed_time_step();
    if new_ts != ts.time_step {
        ts.time_step = new_ts;
    }
}

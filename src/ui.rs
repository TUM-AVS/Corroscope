use bevy::prelude::*;
use bevy_mod_picking::picking_core::PickingPluginsSettings;

pub struct SelectiveInputPlugin;

impl Plugin for SelectiveInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EguiBlockInputState>()
            .add_systems(
                PreUpdate,
                egui_block_input.after(bevy_egui::EguiSet::ProcessInput),
            )
            .add_systems(
                PreUpdate,
                egui_block_picking.before(bevy_mod_picking::picking_core::PickSet::ProcessInput),
            )
            .add_systems(
                Update,
                egui_block_pancam.before(bevy_pancam::PanCamSystemSet),
            )
            .add_systems(
                PostUpdate,
                egui_wants_input.after(bevy_egui::EguiSet::ProcessOutput),
            )
            .add_systems(PreUpdate, update_ui_scale_factor);
    }
}

#[derive(Default, Resource)]
struct EguiBlockInputState {
    wants_keyboard_input: bool,
    wants_pointer_input: bool,
    pointer_over_area: bool,
}

fn egui_wants_input(
    mut state: ResMut<EguiBlockInputState>,
    mut egui_context: bevy_egui::EguiContexts,
    primary_q: Query<Entity, With<bevy::window::PrimaryWindow>>,
) {
    let Ok(window) = primary_q.get_single() else {
        bevy::log::warn!("failed to query primary window");
        return;
    };
    let Some(ctx) = egui_context.try_ctx_for_window_mut(window) else {
        bevy::log::warn!("failed to get egui context for primary window");
        return;
    };

    state.wants_keyboard_input = ctx.wants_keyboard_input();
    state.wants_pointer_input = ctx.wants_pointer_input();
    state.pointer_over_area = ctx.is_pointer_over_area();
}

fn egui_block_input(
    state: Res<EguiBlockInputState>,

    mut keys: ResMut<Input<KeyCode>>,
    mut mouse_buttons: ResMut<Input<MouseButton>>,
) {
    if state.wants_keyboard_input {
        keys.reset_all();
    }
    if state.wants_pointer_input {
        mouse_buttons.reset_all();
    }
}

fn egui_block_pancam(
    state: Res<EguiBlockInputState>,

    mut pancam_q: Query<(Entity, &mut bevy_pancam::PanCam)>,
) {
    let _span = bevy::log::debug_span!(
        "egui disable input for pancam",
        egui_wants_keyboard = state.wants_keyboard_input,
        egui_wants_pointer = state.wants_pointer_input,
        egui_pointer_over_area = state.pointer_over_area,
    )
    .entered();

    if !state.is_changed() {
        return;
    }

    let pancam_enable = !state.wants_pointer_input && !state.pointer_over_area;

    for (entity, mut pancam) in &mut pancam_q {
        bevy::log::debug!(
            "pancam {:?}: setting picking enabled to {}",
            entity,
            pancam_enable
        );
        pancam.enabled = pancam_enable;
    }
}

fn egui_block_picking(
    state: Res<EguiBlockInputState>,

    mut picking_settings: ResMut<PickingPluginsSettings>,
) {
    let _span = bevy::log::debug_span!(
        "egui disable input for picking",
        egui_wants_keyboard = state.wants_keyboard_input,
        egui_wants_pointer = state.wants_pointer_input,
        egui_pointer_over_area = state.pointer_over_area,
    )
    .entered();

    if !state.is_changed() {
        return;
    }

    let picking_enable = !state.wants_pointer_input && !state.pointer_over_area;

    bevy::log::debug!("setting picking enabled to {}", picking_enable);

    picking_settings.enable = picking_enable;
}

fn update_ui_scale_factor(
    mut egui_settings: ResMut<bevy_egui::EguiSettings>,
    windows: Query<&Window, Changed<bevy::window::PrimaryWindow>>,
) {
    if let Ok(window) = windows.get_single() {
        egui_settings.scale_factor = 1.0 / window.scale_factor();
    }
}
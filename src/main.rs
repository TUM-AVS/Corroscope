use bevy::prelude::*;

use bevy_egui::{EguiContexts, EguiPlugin};

#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

#[cfg(feature = "editor")]
use bevy_editor_pls::prelude::*;

use bevy_prototype_lyon::prelude::*;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

use commonroad_pb::CommonRoad;

use prost::Message;
use std::fs::File;
use std::io::Read;

use bevy_mod_picking::prelude::*;

mod conversion;

pub mod commonroad_pb;

pub mod elements;

impl Resource for CommonRoad {}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    // let path = "DEU_Muc-2_1_T-1.pb";
    // let path = "../pb_scenarios/ZAM_Tjunction-1_41_T-1.pb";
    let path = "../pb_scenarios/DEU_Muc-4_1_T-1.pb";
    let f = File::open(path).unwrap();
    // let mut f = File::open("USA_Lanker-1_1_T-1.pb").unwrap();
    let cr = read_cr(f);

    let mut app = App::new();

    app.insert_resource(ClearColor(Color::rgb_u8(105, 105, 105)))
        .insert_resource(Msaa::Sample4)
        .insert_resource(cr)
        .init_resource::<CurrentTimeStep>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Corroscope".into(),
                // present_mode: bevy::window::PresentMode::AutoNoVsync,
                present_mode: bevy::window::PresentMode::AutoVsync,
                // Tells wasm to resize the window according to the available canvas
                fit_canvas_to_parent: true,
                // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        // .add_plugins(DefaultPickingPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(bevy_framepace::FramepacePlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(ShapePlugin)
        .add_plugin(bevy_pancam::PanCamPlugin::default())
        .add_startup_system(camera_setup)
        .add_startup_system(setup)
        .add_startup_system(elements::spawn_trajectories)
        .add_system(elements::highlight_trajectories)
        .add_system(elements::trajectory_tooltip)
        .add_system(elements::obstacle::plot_obs)
        .add_system(elements::obstacle::obstacle_tooltip)
        .add_system(elements::obstacle::trajectory_animation)
        .add_system(side_panel)

        // .add_system(animate_time)
        ;

    app.add_plugins(DefaultPickingPlugins);

    #[cfg(feature = "debug_picking")]
    {
        use bevy_mod_picking::debug::DebugPickingMode::{Disabled, Normal};
        app.insert_resource(State(Disabled)).add_systems(
            (
                (|mut next: ResMut<NextState<_>>| next.set(Normal)).run_if(in_state(Disabled)),
                (|mut next: ResMut<NextState<_>>| next.set(Disabled)).run_if(in_state(Normal)),
            )
                .distributive_run_if(
                    bevy::input::common_conditions::input_just_pressed(KeyCode::F3),
                ),
        );
    }

    #[cfg(feature = "inspector")]
    app.add_plugin(WorldInspectorPlugin::new());

    #[cfg(feature = "editor")]
    app.add_plugin(EditorPlugin::default());

    app.run();

    Ok(())
}

#[derive(Component)]
pub struct MainCamera;

fn camera_setup(mut commands: Commands) {
    commands
        .spawn((
            MainCamera,
            RaycastPickCamera::default(),
            Camera2dBundle {
                projection: OrthographicProjection {
                    scale: 0.1, // 0.001,
                    ..default()
                },
                ..default()
            },
        ))
        .insert(bevy_pancam::PanCam::default());
}

fn read_cr(mut file: std::fs::File) -> commonroad_pb::CommonRoad {
    let mut buffer = Vec::new();

    // read the whole file
    file.read_to_end(&mut buffer).unwrap();

    let buf = bytes::Bytes::from(buffer);

    commonroad_pb::CommonRoad::decode(buf).unwrap()
}

fn setup(mut commands: Commands, cr: Res<CommonRoad>) {
    for obs in &cr.dynamic_obstacles {
        // dbg!(obs);
        elements::obstacle::spawn_obstacle(&mut commands, obs);
    }
    for lanelet in &cr.lanelets {
        elements::lanelet::spawn_lanelet(&mut commands, lanelet);
    }
}

#[derive(Default, Resource)]
pub struct CurrentTimeStep {
    time_step: i32,
    dynamic_time_step: f32,
}

fn animate_time(
    time: Res<Time>,
    mut cts: ResMut<CurrentTimeStep>
) {
    cts.dynamic_time_step += time.delta_seconds() * 7.0;

    if cts.dynamic_time_step > 40.0 {
        cts.dynamic_time_step -= 40.0;
    }
}

fn side_panel(mut contexts: EguiContexts, mut cts: ResMut<CurrentTimeStep>, cr: Res<CommonRoad>) {
    let ctx = contexts.ctx_mut();

    let panel_id = egui::Id::new("side panel left");
    egui::SidePanel::left(panel_id)
        .resizable(false)
        .exact_width(400.0)
        .show(ctx, |ui| {
            ui.heading("Scenario Information");
            ui.label(format!("{:#?}", cr.information));

            ui.heading("Time Control");
            ui.style_mut().spacing.slider_width = 300.0;
            ui.add(
                egui::Slider::new(&mut cts.time_step, 0..=40)
                    .text("time step")
                    .step_by(1.0)
                    .clamp_to_range(true),
            );

            ui.add(
                egui::Slider::new(&mut cts.dynamic_time_step, 0.0..=140.0)
                    .smart_aim(false)
                    .text("d time step")
                    .clamp_to_range(true),
            );
        });
}

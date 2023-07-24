use bevy::prelude::*;

// #[cfg(feature = "inspector")]
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

mod global_settings;

mod args;

impl Resource for CommonRoad {}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    use clap::Parser;
    let args = crate::args::Args::parse();

    // TODO: Improve file loading error reporting
    let f = File::open(&args.scenario).unwrap();
    let cr = read_cr(f);

    let mut app = App::new();

    app.insert_resource(ClearColor(Color::rgb_u8(105, 105, 105)))
        .insert_resource(Msaa::Sample4)
        .insert_resource(cr)
        .insert_resource(args)
        .insert_resource(bevy::winit::WinitSettings::desktop_app())
        .init_resource::<global_settings::GlobalSettings>()
        .init_resource::<global_settings::CurrentTimeStep>()
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
        .add_plugins(DefaultPickingPlugins)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(bevy_egui::EguiPlugin)
        .add_plugins(ShapePlugin)
        .add_plugins(bevy_pancam::PanCamPlugin)
        .add_systems(Startup, (camera_setup, update_ui_scale_factor))
        .add_systems(Update, global_settings::side_panel)
        .add_plugins(elements::ElementsPlugin)
        .add_systems(Update, global_settings::animate_time);

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

    // app
    //     .register_type::<bevy_prototype_lyon::draw::Stroke>()
    //     .register_type::<bevy_prototype_lyon::draw::Fill>();

    // #[cfg(feature = "inspector")]
    // app.add_plugins(WorldInspectorPlugin::new());

    app.add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin);

    #[cfg(feature = "editor")]
    app.add_plugins(EditorPlugin::on_second_monitor_fullscreen(
        EditorPlugin::default(),
    ));

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

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::EguiSettings;

fn update_ui_scale_factor(
    mut egui_settings: ResMut<EguiSettings>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if let Ok(window) = windows.get_single() {
        egui_settings.scale_factor = 1.0 / window.scale_factor();
    }
}

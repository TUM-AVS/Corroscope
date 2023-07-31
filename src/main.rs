use bevy::prelude::*;

#[cfg(feature = "editor")]
use bevy_editor_pls::prelude::*;

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

mod finite;

mod ui;

impl Resource for CommonRoad {}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    use clap::Parser;
    let args = crate::args::Args::parse();

    // TODO: Improve file loading error reporting
    let f = File::open(&args.scenario).unwrap();
    let cr = read_cr(f);

    let mut app = App::new();

    app
        .insert_resource(cr)
        .insert_resource(args)
        .insert_resource(bevy::winit::WinitSettings::desktop_app())
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
            exit_condition: bevy::window::ExitCondition::OnPrimaryClosed,
            close_when_requested: true,
        }))
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin);

    // Rendering
    app
        .insert_resource(ClearColor(Color::rgb_u8(105, 105, 105)))
        .insert_resource(Msaa::Sample4)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(bevy_egui::EguiPlugin)
        .add_plugins(bevy_prototype_lyon::prelude::ShapePlugin)
        .add_plugins(bevy_pancam::PanCamPlugin)
        .add_systems(Startup, (camera_setup, update_ui_scale_factor));

    // Picking
    app
        .add_plugins(DefaultPickingPlugins)
        .insert_resource(bevy_mod_picking::selection::SelectionSettings {
            click_nothing_deselect_all: true,
            use_multiselect_default_inputs: false,
        });

    app
        .add_plugins(global_settings::GlobalSettingsPlugin)
        .add_plugins(elements::ElementsPlugin)
        .add_plugins(ui::SelectiveInputPlugin);

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
    {
        use bevy_inspector_egui::quick::WorldInspectorPlugin;
        app.add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin);

        // app.add_plugins(WorldInspectorPlugin::new());
    }

    #[cfg(feature = "editor")]
    {
        const ENABLE_EDITOR: bool = false;

        if ENABLE_EDITOR {
            app.add_plugins(EditorPlugin::on_second_monitor_fullscreen(
                EditorPlugin::default(),
            ));
        }
    }

    #[cfg(feature = "export_schedule")]
    export_dot(&mut app)?;

    app.run();

    Ok(())
}

fn export_dot(app: &mut App) -> Result<(), std::io::Error> {
    use std::io::Write;

    {
        let settings = bevy_mod_debugdump::render_graph::Settings::default();
        let dot = bevy_mod_debugdump::render_graph_dot( app, &settings);
        let mut dot_file = std::fs::File::create("bevy_render.dot")?;
        dot_file.write_all(dot.as_bytes())?;
    }

    {
        let settings = bevy_mod_debugdump::schedule_graph::Settings::default();
        let dot = bevy_mod_debugdump::schedule_graph_dot(app, Update, &settings);
        let mut dot_file = std::fs::File::create("bevy_schedule.dot")?;
        dot_file.write_all(dot.as_bytes())?;
    }

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

fn update_ui_scale_factor(
    mut egui_settings: ResMut<bevy_egui::EguiSettings>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    if let Ok(window) = windows.get_single() {
        egui_settings.scale_factor = 1.0 / window.scale_factor();
    }
}

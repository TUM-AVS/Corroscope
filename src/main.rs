use bevy::{prelude::*, core::TaskPoolThreadAssignmentPolicy};

use backends::raycast::bevy_mod_raycast::deferred::RaycastSource;

#[cfg(feature = "editor")]
use bevy_editor_pls::prelude::*;

use commonroad_pb::CommonRoad;

use prost::Message;

use bevy_mod_picking::prelude::*;

mod conversion;

pub mod commonroad_pb;

pub mod elements;

mod global_settings;

mod args;

mod finite;

mod ui;

mod extra_shapes;

impl Resource for CommonRoad {}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    // use clap::Parser;
    let args = crate::args::Args::parse();

    let cr = read_cr(&args);

    let mut app = App::new();

    app.insert_resource(cr)
        .insert_resource(args)
        // .insert_resource(bevy::winit::WinitSettings {
        //     focused_mode: bevy::winit::UpdateMode::ReactiveLowPower { wait: Duration::from_secs(5) },
        //     unfocused_mode: bevy::winit::UpdateMode::ReactiveLowPower { wait: Duration::from_secs(90) },
        //     return_from_run: false,
        // })
        .add_plugins(CustomDefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Corroscope".into(),
                // present_mode: bevy::window::PresentMode::AutoNoVsync,
                present_mode: bevy::window::PresentMode::AutoVsync,
                // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            exit_condition: bevy::window::ExitCondition::OnPrimaryClosed,
            close_when_requested: true,
        }));

    #[cfg(feature = "dev")]
    app
        .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin);

    #[cfg(feature = "framepace")]
    app.add_plugins(bevy_framepace::FramepacePlugin);

    // Rendering
    app.insert_resource(ClearColor(Color::rgb_u8(105, 105, 105)))
        .add_plugins(bevy_egui::EguiPlugin)
        .add_plugins(bevy_prototype_lyon::prelude::ShapePlugin)
        // .add_plugins(bevy_polyline::PolylinePlugin)
        .add_plugins(bevy_pancam::PanCamPlugin);

    // Picking
    app.add_plugins(DefaultPickingPlugins).insert_resource(
        bevy_mod_picking::selection::SelectionPluginSettings {
            click_nothing_deselect_all: true,
            use_multiselect_default_inputs: false,
            is_enabled: true,
        },
    );

    app.add_plugins(global_settings::GlobalSettingsPlugin)
        .add_plugins(elements::ElementsPlugin)
        .add_plugins(ui::SelectiveInputPlugin)
        .add_systems(Startup, camera_setup)
        .add_systems(Update, update_camera_3d.after(bevy_pancam::PanCamSystemSet));

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
        app.add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin);

        const ENABLE_WORLD_INSPECTOR: bool = false;
        if ENABLE_WORLD_INSPECTOR {
            use bevy_inspector_egui::quick::WorldInspectorPlugin;
            app.add_plugins(WorldInspectorPlugin::new());
        }
    }

    #[cfg(feature = "editor")]
    {
        const ENABLE_EDITOR: bool = false;
        const EDITOR_SECOND_MONITOR: bool = false;

        if ENABLE_EDITOR {
            let editor = EditorPlugin::default();
            if EDITOR_SECOND_MONITOR {
                app.add_plugins(EditorPlugin::on_second_monitor_fullscreen(editor));
            } else {
                app.add_plugins(editor);
            }
        }
    }

    #[cfg(feature = "export_schedule")]
    export_dot(&mut app)?;

    app.run();

    Ok(())
}

#[cfg(feature = "export_schedule")]
fn export_dot(app: &mut App) -> Result<(), std::io::Error> {
    use std::io::Write;

    {
        let settings = bevy_mod_debugdump::render_graph::Settings::default();
        let dot = bevy_mod_debugdump::render_graph_dot(app, &settings);
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
// #[component(storage = "SparseSet")]
pub(crate) struct MainCamera;

#[derive(Component)]
// #[component(storage = "SparseSet")]
pub(crate) struct MainCamera3d;

pub(crate) fn camera_setup(mut commands: Commands) {
    let ortho = OrthographicProjection {
        scale: 0.1, // 0.001,
        ..default()
    };

    commands.spawn((
        MainCamera,
        RaycastSource::<()>::new_cursor(),
        Camera2dBundle {
            projection: ortho.clone(),
            tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::None,
            ..Camera2dBundle::new_with_far(250.0)
        },
        bevy_pancam::PanCam::default(),
    ));

    commands.spawn((
        MainCamera3d,
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, ortho.far - 0.1).looking_at(Vec3::ZERO, Vec3::Y),
            projection: Projection::Orthographic(ortho.clone()),
            camera: Camera {
                order: 1,
                hdr: false,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::None,
            ..default()
        },
    ));
}

fn update_camera_3d(
    mut camera_main_q: Query<(Ref<Transform>, Ref<OrthographicProjection>), (With<MainCamera>, Without<MainCamera3d>, Or<(Changed<OrthographicProjection>, Changed<Transform>)>)>,

    mut camera_3d_q: Query<(&mut Transform, &mut Projection), (With<MainCamera3d>, Without<MainCamera>)>,
) {
    let Ok((transform, ortho)) = camera_main_q.get_single_mut() else {
        return;
    };

    let (mut transform_3d, mut proj_3d) = camera_3d_q.single_mut();
    if transform.is_changed() {
        transform_3d.set_if_neq(*transform);
    }
    if ortho.is_changed() {
        match proj_3d.as_ref() {
            Projection::Orthographic(ortho_3d) => {
                if ortho_3d.scale != ortho.scale {
                    *proj_3d = Projection::Orthographic(ortho.clone());
                }
            },
            _ => {
                panic!("unexpected projection in MainCamera3d");
            },
        };
   }
}

#[derive(Debug)]
struct MetaQueryError;

impl std::error::Error for MetaQueryError {}

impl std::fmt::Display for MetaQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "meta query error")
    }
}

fn read_cr(args: &crate::args::Args) -> commonroad_pb::CommonRoad {
    let db_path = std::path::Path::join(&args.logs, "trajectories.db");
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
    ).unwrap();

    let data: commonroad_pb::CommonRoad = {
        let mut stmt = conn.prepare(
            "SELECT value FROM meta WHERE key = 'scenario'"
        ).unwrap();

        stmt.query_row([], |row| {
            let rusqlite::types::ValueRef::Blob(st) = row.get_ref(0)? else {
                return Err(rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(MetaQueryError)));
             };

            commonroad_pb::CommonRoad::decode(st).map_err(|err| {
                return rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(err));
            })
        }).unwrap()
    };

    conn.close().unwrap();

    return data;

    /*
    mut file: std::fs::File,
    // TODO: Improve file loading error reporting
    let f = std::fs::File::open(&args.scenario)?;
    let mut buffer = Vec::new();

    // read the whole file
    file.read_to_end(&mut buffer).unwrap();

    let buf = bytes::Bytes::from(buffer);

    commonroad_pb::CommonRoad::decode(buf).unwrap()
    */
}

pub struct CustomDefaultPlugins;

impl PluginGroup for CustomDefaultPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let mut group = bevy::app::PluginGroupBuilder::start::<Self>();
        group = group
            .add(bevy::log::LogPlugin::default())
            .add(bevy::core::TaskPoolPlugin {
                task_pool_options: TaskPoolOptions {
                    min_total_threads: 8,
                    max_total_threads: 8,
                    compute: TaskPoolThreadAssignmentPolicy {
                        min_threads: 7,
                        max_threads: 8,
                        percent: 1.0
                    },
                    ..default()
                }
            })
            .add(bevy::core::TypeRegistrationPlugin::default())
            .add(bevy::core::FrameCountPlugin::default())
            .add(bevy::time::TimePlugin::default())
            .add(bevy::transform::TransformPlugin::default())
            .add(bevy::hierarchy::HierarchyPlugin::default())
            .add(bevy::diagnostic::DiagnosticsPlugin::default())
            .add(bevy::input::InputPlugin::default())
            .add(bevy::window::WindowPlugin::default())
            .add(bevy::a11y::AccessibilityPlugin)
            ;

        {
            group = group.add(bevy::asset::AssetPlugin::default());
        }

        {
            group = group.add(bevy::winit::WinitPlugin::default());
        }

        {
            group = group
                .add(bevy::render::RenderPlugin::default())
                // NOTE: Load this after renderer initialization so that it knows about the supported
                // compressed texture formats
                .add(bevy::render::texture::ImagePlugin::default());

            #[cfg(all(not(target_arch = "wasm32")))]
            {
                group = group
                    .add(bevy::render::pipelined_rendering::PipelinedRenderingPlugin::default());
            }
        }

        {
            group = group.add(bevy::core_pipeline::CorePipelinePlugin::default());
        }

        {
            group = group.add(bevy::sprite::SpritePlugin::default());
        }

        group
    }
}

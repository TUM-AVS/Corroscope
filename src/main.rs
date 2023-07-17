use bevy::prelude::*;

use bevy_editor_pls::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin};

use bevy_inspector_egui::quick::WorldInspectorPlugin;

use bevy_prototype_lyon::prelude::*;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

use commonroad_pb::{integer_exact_or_interval, CommonRoad};
use egui::plot::{PlotBounds, PlotPoints};
use prost::Message;
use std::fs::File;
use std::io::Read;

use bevy_mod_picking::prelude::*;

mod conversion;

pub mod commonroad_pb;

impl Resource for CommonRoad {}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;


    // let path = "DEU_Muc-2_1_T-1.pb";
    // let path = "../pb_scenarios/ZAM_Tjunction-1_41_T-1.pb";
    let path = "../pb_scenarios/DEU_Muc-4_1_T-1.pb";
    let f = File::open(path).unwrap();
    // let mut f = File::open("USA_Lanker-1_1_T-1.pb").unwrap();
    let cr = read_cr(f);
    // cr.information.time_step_size

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
        .add_system(plot_obs)
        .add_system(obstacle_tooltip)
        .add_system(trajectory_animation)
        .add_system(side_panel);

    use bevy_mod_picking::debug::DebugPickingMode::{Normal, Disabled};
    app.add_plugins(DefaultPickingPlugins)
        .insert_resource(State(Disabled))
        .add_systems(
            (
                (|mut next: ResMut<NextState<_>>| next.set(Normal)).run_if(in_state(Disabled)),
                (|mut next: ResMut<NextState<_>>| next.set(Disabled)).run_if(in_state(Normal)),
            )
                .distributive_run_if(bevy::input::common_conditions::input_just_pressed(KeyCode::F3)),
        );

    // #[cfg(debug)]
    // app.add_plugin(WorldInspectorPlugin::new());

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

#[derive(Component)]
pub struct LeftBound;

#[derive(Component)]
pub struct RightBound;

#[derive(Component)]
pub struct LaneletBackground;

#[derive(Component)]
pub struct Lanelet;

fn make_dashed(path: &lyon_path::Path, dash_length: f32, dash_ratio: f32) -> lyon_path::Path {
    use lyon_algorithms::measure::{PathMeasurements, SampleType};

    let measurements = PathMeasurements::from_path(path, 1e-3);
    let mut sampler = measurements.create_sampler(path, SampleType::Distance);
    let rb_length = sampler.length();

    debug_assert!(dash_ratio < 1.0);

    let dash_count = (rb_length / dash_length).ceil() as i32;

    let mut builder = lyon_path::Path::builder();

    for i in 0..dash_count {
        let dash_start = i as f32 * dash_length;
        let dash_end   = i as f32 * dash_length + dash_ratio * dash_length;

        sampler.split_range(dash_start..dash_end, &mut builder);
    }

    builder.build()
}

fn spawn_bound(bound: &commonroad_pb::Bound, id: u32) -> Option<impl Bundle> {
    let bound_pts = bound
        .points
        .iter()
        .map(|p| Vec2::new(p.x as f32, p.y as f32));

    let rb_shape = bevy_prototype_lyon::shapes::Polygon {
        points: bound_pts.collect(),
        closed: false,
    };
    let rb_path = GeometryBuilder::build_as(&rb_shape);

    let dashed_path = Path(make_dashed(&rb_path.0, 0.8, 0.5));
    let short_dashed_path = Path(make_dashed(&rb_path.0, 0.2, 0.5));

    let marking_color = Color::CRIMSON;
    let stroke_opts = {
        let mut opts = StrokeOptions::default();
        opts.start_cap = LineCap::Round;
        opts.end_cap = LineCap::Round;
        opts.line_join = LineJoin::Round;
        opts.line_width = 0.1;
        opts.tolerance = 1e-4;
        opts
    };
    let normal_stroke = Stroke {
        color: marking_color,
        options: stroke_opts, // st 0.1
    };
    let broad_stroke = Stroke {
        color: marking_color,
        options: {
            let mut opts = stroke_opts;
            opts.line_width = 0.2;
            opts
        },
    };
    let light_stroke = Stroke {
        color: Color::LIME_GREEN.with_a(0.5),
        options: {
            let mut opts = stroke_opts;
            opts.line_width = 0.07;
            opts
        },
    };

    use crate::commonroad_pb::line_marking_enum::LineMarking;

    let (path, stroke) = match bound.line_marking() {
        LineMarking::Solid => { (rb_path, normal_stroke) },
        LineMarking::BroadSolid => { (rb_path, broad_stroke) },
        LineMarking::Dashed => { (dashed_path, normal_stroke) },
        LineMarking::BroadDashed => { (dashed_path, normal_stroke) },
        LineMarking::Unknown => {
            bevy::log::info!("lanelet bound has unknown line marking");

            (short_dashed_path, light_stroke)
        },
        LineMarking::NoMarking => {
            bevy::log::info!("lanelet bound has no line marking");

            (short_dashed_path, light_stroke)
        },
    };

    dbg!(id);

    Some((
        ShapeBundle {
            path,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 2.0 + (id as f32 * 1e-2))),
            ..default()
        },
        stroke,
    ))

}

fn spawn_lanelet(commands: &mut Commands, lanelet: &commonroad_pb::Lanelet) {
    if let Some(stop_line) = &lanelet.stop_line {
        let stop_line_shape: shapes::Polygon = bevy_prototype_lyon::shapes::Polygon {
            points: stop_line.points.iter().map(Into::into).collect(),
            closed: false,
        };
        commands.spawn((
            Name::new("stop line"),
            ShapeBundle {
                path: GeometryBuilder::build_as(&stop_line_shape),
                transform: Transform::from_xyz(0.0, 0.0, 1.0),
                ..default()
            },
            Fill::color(Color::BLACK),
        ));
    };

    let lbound_pts = lanelet
        .left_bound
        .points
        .iter()
        .map(|p| Into::<Vec2>::into(p));
    let rbound_pts = lanelet
        .right_bound
        .points
        .iter()
        .map(|p| Into::<Vec2>::into(p));

    let mut fpoints: Vec<Vec2> = vec![];
    fpoints.extend(lbound_pts.clone());
    fpoints.extend(rbound_pts.clone().rev());

    let ll_shape: shapes::Polygon = bevy_prototype_lyon::shapes::Polygon {
        points: fpoints,
        closed: false,
    };

    let main_entity = commands.spawn((
        Name::new("lanelet"),
        Lanelet,
        SpatialBundle::default(),
    )).id();

    commands.spawn((
        Name::new("background"),
        LaneletBackground,
        ShapeBundle {
            path: GeometryBuilder::build_as(&ll_shape),
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        },
        Fill::color(Color::GRAY),
    )).set_parent_in_place(main_entity);

    if let Some(bound) = spawn_bound(&lanelet.left_bound, lanelet.lanelet_id) {
        commands.spawn((
            Name::new("left bound"),
            LeftBound,
            bound,
        )).set_parent(main_entity);
    }
    if let Some(bound) = spawn_bound(&lanelet.right_bound, lanelet.lanelet_id) {
        commands.spawn((
            Name::new("right bound"),
            RightBound,
            bound,
        )).set_parent(main_entity);
    }
/*
    commands.spawn((
        LeftBound,
        ShapeBundle {
            path: GeometryBuilder::build_as(&lb_shape),
            ..default()
        },
        Stroke::new(Color::CYAN, 0.1),
    ));

    commands.spawn((
        RightBound,
        ShapeBundle {
            path: Path(dashed_path), // GeometryBuilder::build_as(&rb_shape),
            ..default()
        },
        Stroke::new(Color::CYAN, 0.1),
    ));
    */
}

fn state_transform(state: &commonroad_pb::State) -> Option<Transform> {
    let position: Vec2 = match state.position.as_ref()? {
        commonroad_pb::state::Position::Point(p) => Vec2::from(p.clone()),
        _ => { return None; },
    };
    let angle = match state.orientation.as_ref()?.exact_or_interval.as_ref()? {
        commonroad_pb::float_exact_or_interval::ExactOrInterval::Exact(e) => *e as f32,
        _ => { return None; },
    };

    let mut t = Transform::from_translation(position.extend(1.0));
    t.rotate_z(angle);
    Some(t)
}

#[derive(Component)]
pub struct ObstacleData(commonroad_pb::DynamicObstacle);

#[derive(Component)]
pub struct HoveredObstacle;

fn obstacle_tooltip(
    mut contexts: EguiContexts,

    obstacle_q: Query<(&ObstacleData, &Transform), With<HoveredObstacle>>,
) {
    let ctx = contexts.ctx_mut();

    let base_id = egui::Id::new("obstacle tooltip");
    for (ObstacleData(obs), transform) in obstacle_q.iter() {
        let tt_pos: egui::Pos2 = obs
            .initial_state
            .position
            .as_ref()
            .unwrap()
            .to_owned()
            .try_into()
            .unwrap();

        egui::containers::show_tooltip(
            ctx,
            base_id.with(obs.dynamic_obstacle_id),
            // Some(tt_pos),
            |ui| {
                ui.heading(format!("Obstacle {} (type {:#?})", obs.dynamic_obstacle_id, obs.obstacle_type()));
                // ui.label(format!("type: {:#?}", obs.obstacle_type()));


                ui.label(format!("signal series: {:#?}", obs.signal_series));
                ui.label(format!("initial signal state: {:#?}", obs.initial_signal_state));

                // ui.label(format!("initial state: {:#?}", obs.initial_state));
            },
        );
    }
}

fn spawn_obstacle(commands: &mut Commands, obs: &commonroad_pb::DynamicObstacle) {
    let shape = match obs.shape.shape.as_ref().unwrap() {
        commonroad_pb::shape::Shape::Rectangle(r) => bevy_prototype_lyon::shapes::Rectangle {
            extents: Vec2::new(r.length as f32, r.width as f32),
            origin: RectangleOrigin::Center,
        },
        _ => unimplemented!(),
    };
    let rect_path: bevy_prototype_lyon::prelude::Path = GeometryBuilder::build_as(&shape);
    let simple_marker = bevy_prototype_lyon::shapes::Circle {
            radius: 0.4,
            center: Vec2::ZERO,
    };

    let _main_entity = commands.spawn((
        Name::new("obstacle"),
        ObstacleData(obs.to_owned()),
        ShapeBundle {
            path: rect_path,
            transform: {
                let mut t = state_transform(&obs.initial_state).unwrap();
                t.translation.z = 4.0;
                t
            },

            ..default()
        },
        Fill::color(Color::WHITE),
        Stroke::new(Color::ORANGE, 0.4),
        PickableBundle::default(),
        RaycastPickTarget::default(),
        On::<Pointer<Down>>::target_commands_mut(|_click, _commands| {
            bevy::log::info!("clicked obstacle!");
        }),
        On::<Pointer<Over>>::target_commands_mut(|_click, commands| {
            commands.insert(HoveredObstacle);
        }),
        On::<Pointer<Out>>::target_commands_mut(|_click, commands| {
            commands.remove::<HoveredObstacle>();
        }),
    )).id();

    let Some(commonroad_pb::dynamic_obstacle::Prediction::TrajectoryPrediction(traj)) = &obs.prediction
        else { return; };

    for st in &traj.trajectory.states {
        let time_step = match st.time_step.exact_or_interval {
            Some(integer_exact_or_interval::ExactOrInterval::Exact(i)) => i,
            _ => unimplemented!(),
        };
        let ts_color = Color::rgba_u8(
            130_u8.saturating_sub((time_step as u8).saturating_mul(2)),
            50,
            140,
            100_u8.saturating_sub((time_step as u8)), //.saturating_mul(4)),
        );

        commands.spawn((
            Name::new(format!("trajectory prediction for t={}", time_step)),
            ShapeBundle {
                path: GeometryBuilder::build_as(&simple_marker),
                transform: state_transform(st)
                    .unwrap()
                    .mul_transform(Transform::from_xyz(0.0, 0.0, 0.5 - (time_step as f32 * 1e-7))),
                ..default()
            },
            Fill::color(ts_color),
        )); //.set_parent(main_entity);
    }
}

fn setup(mut commands: Commands, cr: Res<CommonRoad>) {
    for obs in &cr.dynamic_obstacles {
        // dbg!(obs);
        spawn_obstacle(&mut commands, obs);
    }
    for lanelet in &cr.lanelets {
        spawn_lanelet(&mut commands, lanelet);
    }
}

fn velocity_points(obs: &commonroad_pb::DynamicObstacle) -> Option<PlotPoints> {
    let commonroad_pb::dynamic_obstacle::Prediction::TrajectoryPrediction(traj) = &obs.prediction.as_ref()?
        else { return None; };

    let velocity_pts = traj
        .trajectory
        .states
        .iter()
        .map(|st| {
            let v = st.velocity.clone()?.try_into().ok()?;
            let ts: i32 = st.time_step.clone().try_into().ok()?;
            Some([ts as f64, v])
        })
        .collect::<Option<Vec<_>>>()?;

    Some(PlotPoints::new(velocity_pts))
}

#[derive(Default, Resource)]
struct CurrentTimeStep {
    time_step: i32,
    dynamic_time_step: f32,
}

fn side_panel(mut contexts: EguiContexts, mut cts: ResMut<CurrentTimeStep>, cr: Res<CommonRoad>) {
    let ctx = contexts.ctx_mut();

    let panel_id = egui::Id::new("side panel left");
    egui::SidePanel::left(panel_id)
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

fn lerp_states(states: &Vec<commonroad_pb::State>, s: f32) -> Option<Transform> {
    let idx = s.floor() as usize;
    if (idx + 1) >= states.len() {
        let last_state = states.last()?;
        return state_transform(last_state);
    }

    let s_idx = s.fract();
    let mut f1 = state_transform(&states[idx])?;
    let f2 = state_transform(&states[idx + 1])?;

    f1.translation = f1.translation.lerp(f2.translation, s_idx);
    f1.rotation = f1.rotation.lerp(f2.rotation, s_idx);

    Some(f1)
}

fn trajectory_animation(
    mut obstacle_q: Query<(&ObstacleData, &mut Transform)>,
    cts: Res<CurrentTimeStep>,
) {
    for (obs, mut transform) in obstacle_q.iter_mut() {
        let commonroad_pb::dynamic_obstacle::Prediction::TrajectoryPrediction(traj) = &obs.0.prediction.as_ref().unwrap()
            else { return; };
        let states = &traj.trajectory.states;

        *transform = lerp_states(&states, cts.dynamic_time_step).unwrap();
    }
}

fn plot_obs(mut contexts: EguiContexts, cr: Res<CommonRoad>) {
    let ctx = contexts.ctx_mut();

    egui::Window::new("Obstacle Velocity").show(ctx, |ui| {
        egui::plot::Plot::new("velocity_plot")
            // .clamp_grid(true)
            .legend(egui::plot::Legend::default())
            .view_aspect(2.0)
            .min_size(egui::Vec2::new(400.0, 200.0))
            .sharp_grid_lines(true)
            .show(ui, |pui| {
                for obs in &cr.dynamic_obstacles {
                    let pp = velocity_points(obs).unwrap();
                    let line = egui::plot::Line::new(pp)
                        .name(format!("velocity [m/s] for {}", obs.dynamic_obstacle_id)); //.shape(Mark);
                    pui.line(line);

                    // pui.set_plot_bounds(PlotBounds::from_min_max([1.0, 0.0], [40.0, 20.0]));
                }
            });
    });
}

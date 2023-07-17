use bevy::prelude::*;

use bevy_egui::{EguiPlugin, EguiContexts};
use bevy_editor_pls::prelude::*;

use bevy_inspector_egui::quick::WorldInspectorPlugin;

use bevy_prototype_lyon::prelude::*;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

use commonroad_pb::{integer_exact_or_interval, CommonRoad, float_exact_or_interval};
use egui::plot::{PlotPoints, PlotBounds};
use prost::Message;
use std::fs::File;
use std::io::Read;

use bevy_mod_picking::prelude::*;

pub mod commonroad_pb;

impl Resource for CommonRoad {}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let cr = read_cr();

    let mut app = App::new();

    app
        .insert_resource(ClearColor(Color::rgb_u8(105, 105, 105)))
        .insert_resource(Msaa::Sample4)
        .insert_resource(cr)
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
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(bevy_framepace::FramepacePlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(ShapePlugin)
        .add_plugin(bevy_pancam::PanCamPlugin::default())

        .add_startup_system(camera_setup)
        
        .add_startup_system(setup)

        .add_system(plot_obs)
        ;

    // #[cfg(debug)]
    // app.add_plugin(WorldInspectorPlugin::new());

    app.add_plugin(EditorPlugin::default());

    app
        .run();

    Ok(())
}

#[derive(Component)]
pub struct MainCamera;

fn camera_setup(mut commands: Commands) {
    commands.spawn((
        MainCamera,
        RaycastPickCamera::default(),
        Camera2dBundle {
        projection: OrthographicProjection {
            scale: 0.1, // 0.001,
            ..default()
        },
        ..default()
    })).insert(bevy_pancam::PanCam::default());
}

fn read_cr() -> commonroad_pb::CommonRoad {
    // let mut f = File::open("DEU_Muc-2_1_T-1.pb").unwrap();
    let mut f = File::open("USA_Lanker-1_1_T-1.pb").unwrap();

    let mut buffer = Vec::new();

    // read the whole file
    f.read_to_end(&mut buffer).unwrap();

    let buf = bytes::Bytes::from(buffer);

    commonroad_pb::CommonRoad::decode(buf).unwrap()
}

#[derive(Component)]
pub struct LeftBound;

#[derive(Component)]
pub struct RightBound;

#[derive(Component)]
pub struct LaneletBackground;

fn spawn_lanelet(commands: &mut Commands, lanelet: &commonroad_pb::Lanelet) {
    let lbound_pts = lanelet.left_bound
        .points.iter().map(|p| Vec2::new(p.x as f32, p.y as f32));
    let rbound_pts = lanelet.right_bound
        .points.iter().map(|p| Vec2::new(p.x as f32, p.y as f32));
   
    let mut fpoints = vec![];
    fpoints.extend(lbound_pts.clone());
    fpoints.extend(rbound_pts.clone().rev());
    
    let ll_shape = bevy_prototype_lyon::shapes::Polygon {
        points: fpoints,
        closed: false,
    };

    let lb_shape = bevy_prototype_lyon::shapes::Polygon {
        points: lbound_pts.collect(),
        closed: false,
    };
    let rb_shape = bevy_prototype_lyon::shapes::Polygon {
        points: rbound_pts.collect(),
        closed: false,
    };

    commands.spawn((
        LaneletBackground,
        ShapeBundle {
            path: GeometryBuilder::build_as(&ll_shape),
            ..default()
        },
        Fill::color(Color::GRAY),
    ));

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
            path: GeometryBuilder::build_as(&rb_shape),
            ..default()
        },
        Stroke::new(Color::CYAN, 0.1),
    ));
}

impl From<commonroad_pb::Point> for Vec2 {
    fn from(value: commonroad_pb::Point) -> Self {
        Vec2::new(value.x as f32, value.y as f32)
    }
}

fn state_transform(state: &commonroad_pb::State) -> Option<Transform> {
    let position: Vec2 = match state.position.as_ref()? {
        commonroad_pb::state::Position::Point(p) => {
            Vec2::from(p.clone())
        },
        _ => unimplemented!()
    };
    let angle = match state.orientation.as_ref()?.exact_or_interval.as_ref()? {
        commonroad_pb::float_exact_or_interval::ExactOrInterval::Exact(e) => *e as f32,
        _ => unimplemented!()
    };

    let mut t = Transform::from_translation(position.clone().extend(1.0));
    t.rotate_z(angle);
    Some(t)
}

fn spawn_obstacle(commands: &mut Commands, obs: &commonroad_pb::DynamicObstacle) {
    let shape = match obs.shape.shape.as_ref().unwrap() {
        commonroad_pb::shape::Shape::Rectangle(r) => {
            bevy_prototype_lyon::shapes::Rectangle {
                extents: Vec2::new(r.length as f32, r.width as f32),
                origin: RectangleOrigin::Center,
            }
        },
        _ => unimplemented!()
    };


    commands.spawn((
        ShapeBundle {
            path: GeometryBuilder::build_as(&shape),
            transform: state_transform(&obs.initial_state).unwrap(),

            ..default()
        },
        
        Fill::color(Color::WHITE),
        Stroke::new(Color::ORANGE, 0.4),

        PickableBundle::default(),
        RaycastPickTarget::default(),

        On::<Pointer<Over>>::target_commands_mut(|_click, _commands| {
            bevy::log::info!("clicked obstacle!");
        }),
    ));

    if let Some(commonroad_pb::dynamic_obstacle::Prediction::TrajectoryPrediction(traj)) = &obs.prediction {
        for st in &traj.trajectory.states {
            let time_step = match st.time_step.exact_or_interval {
                Some(integer_exact_or_interval::ExactOrInterval::Exact(i)) => i,
                _ => unimplemented!()
            };
            let ts_color = Color::rgba_u8(
                130_u8.saturating_sub( (time_step as u8).saturating_mul(2) ),
                50,
                140,
                100_u8.saturating_sub( (time_step as u8).saturating_mul(4) )
            );
            
            commands.spawn((
                ShapeBundle {
                    path: GeometryBuilder::build_as(&shape),
                    transform: state_transform(&st).unwrap(),
                    ..default()
                },
                Fill::color(ts_color),
            ));
        }
    }
}

fn setup(mut commands: Commands) {
    let cr = read_cr();

    for obs in &cr.dynamic_obstacles {
        // dbg!(obs);
        spawn_obstacle(&mut commands, obs);
    }
    for lanelet in &cr.lanelets {
        spawn_lanelet(&mut commands, lanelet);
    }
}

impl TryFrom<commonroad_pb::FloatExactOrInterval> for f64 {
    type Error = ();

    fn try_from(value: commonroad_pb::FloatExactOrInterval) -> Result<Self, Self::Error> {
        TryFrom::try_from(value.exact_or_interval.ok_or(())?)
    }
}

impl TryFrom<commonroad_pb::float_exact_or_interval::ExactOrInterval> for f64 {
    type Error = ();

    fn try_from(value: commonroad_pb::float_exact_or_interval::ExactOrInterval) -> Result<Self, Self::Error> {
        match value {
            float_exact_or_interval::ExactOrInterval::Exact(e) => Ok(e),
            _ => Err(()) ,
        }
    }
}


impl TryFrom<commonroad_pb::IntegerExactOrInterval> for i32 {
    type Error = ();

    fn try_from(value: commonroad_pb::IntegerExactOrInterval) -> Result<Self, Self::Error> {
        TryFrom::try_from(value.exact_or_interval.ok_or(())?)
    }
}

impl TryFrom<commonroad_pb::integer_exact_or_interval::ExactOrInterval> for i32 {
    type Error = ();

    fn try_from(value: commonroad_pb::integer_exact_or_interval::ExactOrInterval) -> Result<Self, Self::Error> {
        match value {
            integer_exact_or_interval::ExactOrInterval::Exact(e) => Ok(e),
            _ => Err(()) ,
        }
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

fn plot_obs(
    mut contexts: EguiContexts,
    cr: Res<CommonRoad>,
) {
    let ctx = contexts.ctx_mut();

    let first_obs = cr.dynamic_obstacles.iter().next().unwrap();
    let pp = velocity_points(first_obs).unwrap();

    egui::containers::show_tooltip(
        ctx,
        egui::Id::new("obstacle tooltip"),
        |ui| {
            ui.label("hello world!");
        });

    egui::Window::new("Obstacle Velocity")
        .show(ctx, |ui| {
        egui::plot::Plot::new("velocity_plot")
            // .clamp_grid(true)
            .legend(egui::plot::Legend::default())
            
            .show(ui, |pui| {
                for obs in &cr.dynamic_obstacles {
                    let pp = velocity_points(&obs).unwrap();
                    let line = egui::plot::Line::new(pp)
                        .name(format!("velocity [m/s] for {}", obs.dynamic_obstacle_id))
                        
                        
                        ; //.shape(Mark);
                    pui.line(line);

                    pui.set_plot_bounds(PlotBounds::from_min_max([1.0, 0.0], [40.0, 20.0]));
                    // pui

                    //pui.points(points);
                }

            });
    });
}


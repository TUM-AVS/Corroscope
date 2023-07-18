use bevy::prelude::*;

use bevy_egui::EguiContexts;

use bevy_prototype_lyon::prelude::*;

use crate::commonroad_pb;

use crate::commonroad_pb::{integer_exact_or_interval, CommonRoad};
use egui::plot::PlotPoints;

use bevy_mod_picking::prelude::*;


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

pub fn obstacle_tooltip(
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

pub fn spawn_obstacle(commands: &mut Commands, obs: &commonroad_pb::DynamicObstacle) {
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

pub fn trajectory_animation(
    mut obstacle_q: Query<(&ObstacleData, &mut Transform)>,
    cts: Res<crate::CurrentTimeStep>,
) {
    for (obs, mut transform) in obstacle_q.iter_mut() {
        let commonroad_pb::dynamic_obstacle::Prediction::TrajectoryPrediction(traj) = &obs.0.prediction.as_ref().unwrap()
            else { return; };
        let states = &traj.trajectory.states;

        *transform = lerp_states(states, cts.dynamic_time_step).unwrap();
    }
}

pub fn plot_obs(mut contexts: EguiContexts, cr: Res<CommonRoad>) {
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

use bevy::prelude::*;

use bevy_mod_picking::prelude::*;
use bevy_prototype_lyon::prelude::*;

use bevy_egui::EguiContexts;

use crate::CurrentTimeStep;

pub mod lanelet;
pub mod obstacle;

fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let s: &str = serde::Deserialize::deserialize(deserializer)?;

    s.to_lowercase().parse::<bool>().map_err(D::Error::custom)
}

fn deserialize_float_list<'de, D>(deserializer: D) -> Result<Vec<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let s: &str = serde::Deserialize::deserialize(deserializer)?;

    s.split(',')
        .map(|s| s.parse::<f64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(D::Error::custom)
}

#[allow(dead_code, non_snake_case)]
#[derive(Debug, serde::Deserialize, Clone, Component, Reflect)]
pub struct Costs {
    Acceleration_cost: f64,
    Distance_to_Obstacles_cost: f64,
    Distance_to_Reference_Path_cost: f64,
    Jerk_cost: f64,
    Lane_Center_Offset_cost: f64,
    Lateral_Jerk_cost: f64,
    Longitudinal_Jerk_cost: f64,
    Occ_PM_cost: f64,
    Occ_UM_cost: f64,
    Occ_VE_cost: f64,
    Orientation_Offset_cost: f64,
    Path_Length_cost: f64,
    Prediction_cost: f64,
    Responsibility_cost: f64,
    Velocity_Costs_cost: f64,
    Velocity_Offset_cost: f64,
}

#[allow(dead_code, non_snake_case)]
#[derive(Debug, serde::Deserialize, Clone, Component, Reflect)]
pub struct TrajectoryLog {
    // time_step;trajectory_number;unique_id;feasible;horizon;dt;actual_traj_length;
    // x_positions_m;y_positions_m;theta_orientations_rad;velocities_mps;accelerations_mps2;s_position_m;d_position_m;
    // costs_cumulative_weighted;Acceleration_cost;Distance_to_Obstacles_cost;Distance_to_Reference_Path_cost;Jerk_cost;Lane_Center_Offset_cost;Lateral_Jerk_cost;Longitudinal_Jerk_cost;Occ_PM_cost;Occ_UM_cost;Occ_VE_cost;Orientation_Offset_cost;Path_Length_cost;Prediction_cost;Responsibility_cost;Velocity_Costs_cost;Velocity_Offset_cost;
    time_step: i32,
    trajectory_number: i32,
    unique_id: i32,
    #[serde(deserialize_with = "deserialize_bool")]
    feasible: bool,
    horizon: f64,
    dt: f64,
    actual_traj_length: f64,

    #[serde(deserialize_with = "deserialize_float_list")]
    x_positions_m: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    y_positions_m: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    theta_orientations_rad: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    velocities_mps: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    accelerations_mps2: Vec<f64>,

    s_position_m: f64,
    d_position_m: f64,

    ego_risk: Option<f64>,
    obst_risk: Option<f64>,
    costs_cumulative_weighted: f64,

    #[serde(flatten)]
    costs: Costs,

    inf_kin_yaw_rate: f64,
    inf_kin_acceleration: f64,
    inf_kin_max_curvature: f64,
    // inf_kin_max_curvature_rate: f64,
}

#[allow(dead_code, non_snake_case)]
#[derive(Debug, serde::Deserialize, Clone, Component, Reflect)]
pub struct MainLog {
    trajectory_number: i32,
    calculation_time_s: f64,
    x_position_vehicle_m: f64,
    y_position_vehicle_m: f64,
    optimal_trajectory: String,
    percentage_feasible_traj: Option<f64>,
    infeasible_kinematics_sum: f64,
    inf_kin_acceleration: f64,
    inf_kin_negative_s_velocity: f64,
    inf_kin_max_s_idx: f64,
    inf_kin_negative_v_velocity: f64,
    inf_kin_max_curvature: f64,
    inf_kin_yaw_rate: f64,
    inf_kin_max_curvature_rate: f64,
    inf_kin_vehicle_acc: f64,
    inf_cartesian_transform: f64,
    infeasible_collision: f64,

    #[serde(deserialize_with = "deserialize_float_list")]
    x_positions_m: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    y_positions_m: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    theta_orientations_rad: Vec<f64>,

    #[serde(deserialize_with = "deserialize_float_list")]
    velocities_mps: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    accelerations_mps2: Vec<f64>,

    // #[serde(deserialize_with = "deserialize_float_list")]
    s_position_m: f64,
    // #[serde(deserialize_with = "deserialize_float_list")]
    d_position_m: f64,

    ego_risk: f64,
    obst_risk: f64,
    costs_cumulative_weighted: f64,

    Acceleration_cost: f64,
    Distance_to_Obstacles_cost: f64,
    Distance_to_Reference_Path_cost: f64,
    Jerk_cost: f64,
    Lane_Center_Offset_cost: f64,
    Lateral_Jerk_cost: f64,
    Longitudinal_Jerk_cost: f64,
    Occ_PM_cost: f64,
    Occ_UM_cost: f64,
    Occ_VE_cost: f64,
    Orientation_Offset_cost: f64,
    Path_Length_cost: f64,
    Prediction_cost: f64,
    Responsibility_cost: f64,
    Velocity_Costs_cost: f64,
    Velocity_Offset_cost: f64,
}

pub fn read_log() -> Result<Vec<TrajectoryLog>, Box<dyn std::error::Error>> {
    let path = "../commonroad-reactive-planner/logs/ZAM_Tjunction-1_100_T-1/trajectories.csv";
    let mut rdr = csv::ReaderBuilder::new()
        // .has_headers(false)
        .delimiter(b';')
        // .flexible(true)
        .from_path(path)?;

    let res = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;

    Ok(res)
}

#[derive(Component)]
pub struct HoveredTrajectory;

pub fn spawn_trajectories(mut commands: Commands) {
    let trajectories = read_log().unwrap();

    for traj in trajectories.as_slice() {
        let points = std::iter::zip(traj.x_positions_m.as_slice(), traj.y_positions_m.as_slice())
            .map(|(&x, &y)| Vec2::new(x as f32, y as f32))
            .collect();

        let stop_line_shape: shapes::Polygon = bevy_prototype_lyon::shapes::Polygon {
            points,
            closed: false,
        };

        // dbg!(traj.time_step);

        let infeasible_color = Color::rgba_u8(
            30, 70, 190, //.saturating_add(traj.unique_id),
            100, //100_u8.saturating_sub((traj.time_step as u8)), //.saturating_mul(4)),
        );

        let ts_color = Color::rgba_u8(
            170_u8.saturating_sub((traj.time_step as u8).saturating_mul(3)),
            60,
            90_u8, //.saturating_add(traj.unique_id),
            100,   //100_u8.saturating_sub((traj.time_step as u8)), //.saturating_mul(4)),
        );
        // dbg!(ts_color);

        commands.spawn((
            Name::new(format!("trajectory {}", traj.trajectory_number)),
            traj.to_owned(),
            ShapeBundle {
                path: GeometryBuilder::build_as(&stop_line_shape),
                transform: Transform::from_xyz(0.0, 0.0, 4.0 + (traj.unique_id as f32) * 1e-6),
                ..default()
            },
            Stroke::new(
                if traj.feasible {
                    ts_color
                } else {
                    infeasible_color
                }, // Color::BLUE.with_a(0.5),
                0.02,
            ),
            On::<Pointer<Over>>::target_commands_mut(|_click, commands| {
                commands.insert(HoveredTrajectory);
            }),
            On::<Pointer<Out>>::target_commands_mut(|_click, commands| {
                commands.remove::<HoveredTrajectory>();
            }),
            PickableBundle::default(),
            RaycastPickTarget::default(),
        ));
    }
}

pub fn highlight_trajectories(
    mut trajectory_q: Query<(&TrajectoryLog, &mut Visibility)>,

    cts: Res<CurrentTimeStep>,
) {
    for (traj, mut visibility) in trajectory_q.iter_mut() {
        if traj.feasible && traj.time_step == cts.dynamic_time_step.round() as i32 {
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

pub fn trajectory_tooltip(
    mut contexts: EguiContexts,

    obstacle_q: Query<(&TrajectoryLog, &Transform), With<HoveredTrajectory>>,

    cts: Res<CurrentTimeStep>,
) {
    let ctx = contexts.ctx_mut();

    let base_id = egui::Id::new("traj tooltip");

    /*
    egui::Window::new("Costs")
        .show(ctx, |ui| {
            egui::plot::Plot::new("cost_plot")
                .show(ui, |pui| {
                    egui::plot::BarChart::new(bars)
                });
        }
        );
    */

    if obstacle_q.is_empty() { return; }
    // if obstacle_q.
    egui::containers::show_tooltip(
        ctx,
        base_id, //.with(traj.unique_id),
        // Some(tt_pos),
        |ui| {
            for (traj, transform) in obstacle_q.iter() {
                ui.heading(format!("Trajectory {}", traj.trajectory_number));
                // ui.label(format!("type: {:#?}", obs.obstacle_type()));

                ui.label(format!("feasible: {}", traj.feasible));

                ui.label(format!("total cost: {}", traj.costs_cumulative_weighted));
                ui.label(format!("collision cost: {}", traj.costs.Prediction_cost));
                ui.label(format!("inf_kin_yaw_rate: {}", traj.inf_kin_yaw_rate));
                ui.label(format!(
                    "inf_kin_acceleration: {}",
                    traj.inf_kin_acceleration
                ));
                ui.label(format!(
                    "inf_kin_max_curvature: {}",
                    traj.inf_kin_max_curvature
                ));
                //ui.label(format!(
                //    "inf_kin_max_curvature_rate: {}",
                //    traj.inf_kin_max_curvature_rate
                //));


                ui.heading("Velocity");
                let velocity_plot = egui::plot::Plot::new("velocity_plot")
                    .view_aspect(2.0)
                    .min_size(egui::Vec2::new(150.0, 75.0))
                    .sharp_grid_lines(true)
                    .include_x(0.0)
                    .include_y(0.0)
                    .height(250.0)
                    .y_grid_spacer(egui::plot::uniform_grid_spacer(|_grid_input| { [10.0, 2.0, 0.5] }))
                    .show(ui, |pui| {
                        let data = traj
                            .velocities_mps
                            .iter()
                            .enumerate()
                            .map(|(x, y)| [(traj.time_step + x as i32) as f64, *y])
                            .collect();
                        let pp = egui::plot::PlotPoints::new(data);
                        let line = egui::plot::Line::new(pp)
                            .name("velocity [m/s]");
                        pui.line(line);

                        pui.vline(egui::plot::VLine::new(cts.dynamic_time_step.round()));
                    });

                ui.heading("Acceleration");
                let _acceleration_plot = egui::plot::Plot::new("acceleration_plot")
                    .view_aspect(2.0)
                    .min_size(egui::Vec2::new(150.0, 75.0))
                    .sharp_grid_lines(true)
                    .center_y_axis(true)
                    .include_x(0.0)
                    .include_y(0.0)
                    .height(250.0)
                    // .link_cursor(velocity_plot, true, false)
                    .show(ui, |pui| {
                        let data = traj
                            .accelerations_mps2
                            .iter()
                            .enumerate()
                            .map(|(x, y)| [(traj.time_step + x as i32) as f64, *y])
                            .collect();
                        let pp = egui::plot::PlotPoints::new(data);
                        let line = egui::plot::Line::new(pp)
                            .name("acceleration [m/s^2]");
                        pui.line(line);

                        pui.vline(egui::plot::VLine::new(cts.dynamic_time_step.round()));
                    });

                let curvature_lim = 0.20855645753;

                ui.heading("Orientation");
                let _theta_plot = egui::plot::Plot::new("theta_plot")
                    .view_aspect(2.0)
                    .min_size(egui::Vec2::new(150.0, 75.0))
                    .sharp_grid_lines(true)
                    .center_y_axis(true)
                    .include_x(0.0)
                    .include_y(0.0)
                    .height(250.0)
                    .show(ui, |pui| {
                        let data = traj
                            .theta_orientations_rad
                            .iter()
                            .enumerate()
                            .map(|(x, y)| [(traj.time_step + x as i32) as f64, *y])
                            .collect();
                        let pp = egui::plot::PlotPoints::new(data);
                        let line = egui::plot::Line::new(pp)
                            .name("orientation [rad]");
                        pui.line(line);

                        pui.vline(egui::plot::VLine::new(cts.dynamic_time_step.round()));
                    });

            }
        },
    );
}

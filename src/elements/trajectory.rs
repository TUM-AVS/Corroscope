use bevy::prelude::*;

use bevy_mod_picking::prelude::*;
use bevy_prototype_lyon::prelude::*;

use bevy_egui::EguiContexts;
use egui::plot::PlotPoint;

use crate::global_settings::CurrentTimeStep;

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

    let s: std::borrow::Cow<'static, str> = serde::Deserialize::deserialize(deserializer)?;

    s
        .split(',')
        .map(|s| s.parse::<f64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(D::Error::custom)
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize, Clone, Default, Reflect)]
pub struct Costs {
    Occ_PM_cost: f64,
    Occ_UM_cost: f64,
    Occ_VE_cost: f64,
    acceleration_cost: f64,
    distance_to_obstacles_cost: f64,
    distance_to_reference_path_cost: f64,
    jerk_cost: f64,
    lane_center_offset_cost: f64,
    lateral_jerk_cost: f64,
    longitudinal_jerk_cost: f64,
    orientation_offset_cost: f64,
    path_length_cost: f64,
    prediction_cost: f64,
    responsibility_cost: f64,
    velocity_cost: f64,
    velocity_offset_cost: f64,
}

#[derive(Debug, serde::Deserialize, Clone, Default, Reflect)]
pub struct KinematicData {
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
}

impl KinematicData {
    fn positions(&self) -> impl Iterator<Item = Vec2> + '_ {
        std::iter::zip(self.x_positions_m.as_slice(), self.y_positions_m.as_slice())
            .map(|(&x, &y)| Vec2::new(x as f32, y as f32))
    }

    fn make_plot_data(data: &Vec<f64>, shift: Option<i32>) -> Vec<[f64; 2]> {
        let shift = shift.unwrap_or(0);

        let pdata = data
            .iter()
            .enumerate()
            .map(|(x, y)| [(shift + x as i32) as f64, *y])
            .collect();

        pdata
    }

    fn velocity_plot_data(&self, shift: Option<i32>) -> Vec<[f64; 2]> {
        Self::make_plot_data(&self.velocities_mps, shift)
    }

    fn acceleration_plot_data(&self, shift: Option<i32>) -> Vec<[f64; 2]> {
        Self::make_plot_data(&self.accelerations_mps2, shift)
    }

    fn orientation_plot_data(&self, shift: Option<i32>) -> Vec<[f64; 2]> {
        Self::make_plot_data(&self.theta_orientations_rad, shift)
    }
}

#[derive(Debug, serde::Deserialize, Clone, Component, Default, Reflect)]
#[reflect(Component)]
pub struct TrajectoryLog {
    time_step: i32,
    trajectory_number: i32,
    unique_id: i32,
    #[serde(deserialize_with = "deserialize_bool")]
    feasible: bool,
    horizon: f64,
    dt: f64,
    actual_traj_length: f64,

    #[serde(flatten)]
    kinematic_data: KinematicData,

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

impl TrajectoryLog {
    fn color(&self) -> Color {
        if self.feasible {
            Color::rgba_u8(
                170_u8.saturating_sub((self.time_step as u8).saturating_mul(3)),
                60,
                90_u8, //.saturating_add(traj.unique_id),
                40,   //100_u8.saturating_sub((traj.time_step as u8)), //.saturating_mul(4)),
            )
        } else {
            Color::rgba_u8(
                30, 70, 190, //.saturating_add(traj.unique_id),
                100, //100_u8.saturating_sub((traj.time_step as u8)), //.saturating_mul(4)),
            )
        }
    }

    fn selected_color(&self) -> Color {
        let base_color = self.color().as_hsla();
        base_color + Color::hsla(0.0, 0.1, 0.3, 0.2)
    }

    fn velocity_plot_data(&self) -> Vec<[f64; 2]> {
        self.kinematic_data.velocity_plot_data(Some(self.time_step))
    }

    fn acceleration_plot_data(&self) -> Vec<[f64; 2]> {
        self.kinematic_data.acceleration_plot_data(Some(self.time_step))
    }

    fn orientation_plot_data(&self) -> Vec<[f64; 2]> {
        self.kinematic_data.orientation_plot_data(Some(self.time_step))
    }
}

#[allow(dead_code, non_snake_case)]
#[derive(Debug, serde::Deserialize, Clone, Component, Default, Reflect)]
#[reflect(Component)]
pub struct MainLog {
    trajectory_number: i32,
    calculation_time_s: f64,
    x_position_vehicle_m: f64,
    y_position_vehicle_m: f64,
    #[serde(deserialize_with = "deserialize_bool")]
    optimal_trajectory: bool,
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

    #[serde(flatten)]
    kinematic_data: KinematicData,

    s_position_m: f64,
    d_position_m: f64,

    ego_risk: Option<f64>,
    obst_risk: Option<f64>,
    costs_cumulative_weighted: f64,

    #[serde(flatten)]
    costs: Costs,
}

pub fn read_log(path: &std::path::Path) -> Result<Vec<TrajectoryLog>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;

    let map = unsafe {
        memmap2::MmapOptions::new()
            .populate()
            .map(&file)?
    };
    let cursor = std::io::Cursor::new(map);


    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(cursor);

    let res = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;

    Ok(res)
}

pub fn read_main_log(path: &std::path::Path) -> Result<Vec<MainLog>, Box<dyn std::error::Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_path(path)?;

    let res = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;

    Ok(res)
}

#[derive(Resource)]
pub struct MainTrajectory {
    path: Vec<Vec2>,
    kinematic_data: KinematicData,
}

#[derive(Component)]
pub struct HoveredTrajectory;

fn reassemble_main_trajectory(mtraj: &Vec<MainLog>) -> KinematicData {
    let x_positions_m = mtraj
        .iter()
        .map(|traj| {
            traj.x_position_vehicle_m
        })
        .collect::<Vec<f64>>();
    let y_positions_m = mtraj
        .iter()
        .map(|traj| {
            traj.y_position_vehicle_m
        })
        .collect::<Vec<f64>>();

    let velocities_mps = mtraj
        .iter()
        .map(|traj| {
            traj.kinematic_data.velocities_mps.first().copied()
        })
        .collect::<Option<Vec<f64>>>()
        .unwrap();

    let accelerations_mps2 = mtraj
        .iter()
        .map(|traj| {
            traj.kinematic_data.accelerations_mps2.first().copied()
        })
        .collect::<Option<Vec<f64>>>()
        .unwrap();

    let theta_orientations_rad = mtraj
        .iter()
        .map(|traj| {
            traj.kinematic_data.theta_orientations_rad.first().copied()
        })
        .collect::<Option<Vec<f64>>>()
        .unwrap();

    KinematicData { x_positions_m, y_positions_m, theta_orientations_rad, velocities_mps, accelerations_mps2 }
}

#[derive(Default, Component)]
pub struct PointerTimeStep {
    time_step: Option<f64>,
}

#[derive(Component, Reflect)]
pub(crate) struct TrajectoryGroup {
    time_step: i32,
}

pub fn spawn_trajectories(mut commands: Commands, args: Res<crate::args::Args>) {
    let trajectories_path = std::path::Path::join(&args.logs, "trajectories.csv");
    let trajectories = {
        let mut v = read_log(&trajectories_path).expect("could not read trajectory logs");
        v.sort_by_key(|t| t.time_step);
        v
    };

    use itertools::Itertools;

    for (ts, traj_group) in &trajectories.into_iter().group_by(|t| t.time_step) {
        let ts_entity = commands.spawn((
            Name::new(format!("trajectory group ts={}", ts)),
            TrajectoryGroup { time_step: ts },
            SpatialBundle::default(),
        )).id();

        for traj in traj_group {
            let points = traj.kinematic_data.positions().collect();

            let traj_shape: shapes::Polygon = bevy_prototype_lyon::shapes::Polygon {
                points,
                closed: false,
            };

            commands.spawn((
                Name::new(format!("trajectory {}", traj.trajectory_number)),
                traj.to_owned(),
                ShapeBundle {
                    path: GeometryBuilder::build_as(&traj_shape),
                    transform: Transform::from_xyz(0.0, 0.0, 4.0 + (traj.unique_id as f32) * 1e-6),
                    ..default()
                },
                Stroke::new(
                    traj.color(),
                    0.05,
                ),
                On::<Pointer<Over>>::target_commands_mut(|_click, commands| {
                    commands.insert(HoveredTrajectory);
                }),
                On::<Pointer<Out>>::target_commands_mut(|_click, commands| {
                    commands.remove::<HoveredTrajectory>();
                }),
                PickableBundle::default(),

                RaycastPickTarget::default(),
            )).set_parent(ts_entity);
        }
    }

    let main_trajectories_path = std::path::Path::join(&args.logs, "logs.csv");
    let main_trajectories = read_main_log(&main_trajectories_path).expect("could not read trajectory logs");
    let mpoints = main_trajectories
        .iter()
        .map(|traj| {
            traj.kinematic_data.positions().next()
        })
        .collect::<Option<Vec<Vec2>>>()
        .unwrap();

    let traj_shape: shapes::Polygon = bevy_prototype_lyon::shapes::Polygon {
        points: mpoints.clone(),
        closed: false,
    };

    commands.insert_resource(MainTrajectory {
        path: mpoints,
        kinematic_data: reassemble_main_trajectory(&main_trajectories),
    });


    commands.spawn((
        Name::new("main trajectory"),
        ShapeBundle {
            path: GeometryBuilder::build_as(&traj_shape),
            transform: Transform::from_xyz(0.0, 0.0, 0.5),
            ..default()
        },
        Stroke::new(
            Color::rgba(0.4, 0.6, 0.18, 0.7),
            0.15,
        ),
    ));


}

pub(crate) fn trajectory_group_visibility(
    mut trajectory_q: Query<(&TrajectoryGroup, &mut Visibility)>,

    cts: Res<CurrentTimeStep>,
) {
    let time_step = cts.dynamic_time_step.round() as i32;

    if !cts.is_changed() {
        return;
    }

    for (traj, mut visibility) in trajectory_q.iter_mut() {
        if traj.time_step == time_step {
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

pub fn trajectory_visibility(
    mut trajectory_q: Query<(&TrajectoryLog, &mut Visibility)>,

    settings: Res<crate::global_settings::GlobalSettings>,
) {
    if !settings.is_changed() {
        return;
    }

    for (traj, mut visibility) in trajectory_q.iter_mut() {
        if traj.feasible || settings.show_infeasible {
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

pub fn trajectory_cursor(
    mut trajectory_q: Query<(&PointerTimeStep, &mut Visibility), Changed<PointerTimeStep>>,

    settings: Res<crate::global_settings::GlobalSettings>,
) {
    let (pointerts, mut visibility) = trajectory_q.single_mut();
    match pointerts.time_step {
        None => {
            *visibility = Visibility::Hidden;
        },
        Some(_ts) => {
            *visibility = Visibility::Visible;
        },
    }
}

pub fn trajectory_tooltip(
    mut contexts: EguiContexts,

    trajectory_q: Query<&TrajectoryLog, With<HoveredTrajectory>>,
) {
    let ctx = contexts.ctx_mut();

    let base_id = egui::Id::new("traj tooltip");

    if trajectory_q.is_empty() { return; }

    egui::containers::show_tooltip(
        ctx,
        base_id, //.with(traj.unique_id),
        // Some(tt_pos),
        |ui| {
            for traj in trajectory_q.iter() {
                ui.heading(format!("Trajectory {}", traj.trajectory_number));
                // ui.label(format!("type: {:#?}", obs.obstacle_type()));

                ui.label(format!("feasible: {}", traj.feasible));

                ui.label(format!("total cost: {}", traj.costs_cumulative_weighted));
                ui.label(format!("collision cost: {}", traj.costs.prediction_cost));
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
                // plot_traj(traj, ui, cts.dynamic_time_step.round());


            }
        },
    );
}

struct TrajectoryPlotData {
    velocity: egui::plot::Line,
    velocity_ref: egui::plot::Line,
    acceleration: egui::plot::Line,
    acceleration_ref: egui::plot::Line,
    orientation: egui::plot::Line,
    orientation_ref: egui::plot::Line,
}

#[derive(Resource, Clone)]
pub struct CachedTrajectoryPlotData {
    time_step: i32,
    trajectory_number: i32,
    unique_id: i32,
    velocity: Vec<[f64; 2]>,
    velocity_ref: Vec<[f64; 2]>,
    acceleration: Vec<[f64; 2]>,
    acceleration_ref: Vec<[f64; 2]>,
    orientation: Vec<[f64; 2]>,
    orientation_ref: Vec<[f64; 2]>,
}


impl CachedTrajectoryPlotData {
    fn from_trajectory(mtraj: &MainTrajectory, traj: &TrajectoryLog) -> Self {
        let velocity = traj.velocity_plot_data();
        let velocity_ref = mtraj.kinematic_data.velocity_plot_data(None);

        let acceleration = traj.acceleration_plot_data();
        let acceleration_ref = mtraj.kinematic_data.acceleration_plot_data(None);
        let orientation = traj.orientation_plot_data();
        let orientation_ref = mtraj.kinematic_data.orientation_plot_data(None);

        Self {
            time_step: traj.time_step,
            trajectory_number: traj.trajectory_number,
            unique_id: traj.unique_id,
            velocity,
            velocity_ref,
            acceleration,
            acceleration_ref,
            orientation,
            orientation_ref,
        }
    }
}

impl TrajectoryPlotData {
    fn from_data(plot_data: CachedTrajectoryPlotData) -> Self {
        let velocity = egui::plot::Line::new(plot_data.velocity)
            .name("velocity");

        let velocity_ref = egui::plot::Line::new(plot_data.velocity_ref)
            .name("ref velocity");

        let acceleration = egui::plot::Line::new(plot_data.acceleration)
            .name("acceleration");

        let acceleration_ref = egui::plot::Line::new(plot_data.acceleration_ref)
            .name("ref acceleration");

        let orientation = egui::plot::Line::new(plot_data.orientation)
            .name("orientation [rad]");

        let orientation_ref = egui::plot::Line::new(plot_data.orientation_ref)
            .style(egui::plot::LineStyle::Dotted { spacing: 6.0 })
            .name("reference orientation [rad]");

        Self {
            velocity,
            velocity_ref,
            acceleration,
            acceleration_ref,
            orientation,
            orientation_ref,
        }
    }
}

fn plot_traj(plot_data: TrajectoryPlotData, ui: &mut egui::Ui, time_step: f32) -> Option<f64> {
    let mut cursor_x = None;

    let group = egui::Id::new("trajectory plot group");

    let plot = |name: &'static str| {
        egui::plot::Plot::new(name)
            .legend(egui::plot::Legend::default().position(egui::plot::Corner::LeftBottom))
            .view_aspect(2.0)
            .min_size(egui::Vec2::new(150.0, 75.0))
            .sharp_grid_lines(true)
            .include_x(0.0)
            .include_y(0.0)
            .height(250.0)
            .link_cursor(group, true, false)
    };

    let unit_label_formatter = |unit: &'static str| {
        move |name: &str, value: &PlotPoint| {
            if !name.is_empty() {
                format!("{}:\n{:.*} {}", name, 1, value.y, unit)
            } else {
                "".to_owned()
            }
        }
    };

    let ts_vline = egui::plot::VLine::new(time_step)
        // .name("current time step")
        .style(egui::plot::LineStyle::Dotted { spacing: 0.1 });

    ui.heading("Velocity");
    let _velocity_plot = plot("velocity_plot")
        .y_grid_spacer(egui::plot::uniform_grid_spacer(|_grid_input| { [10.0, 2.0, 0.5] }))
        .label_formatter(unit_label_formatter("m/s"))
        .show(ui, |pui| {
            pui.line(plot_data.velocity);
            pui.line(plot_data.velocity_ref);

            pui.vline(ts_vline.clone());

            // TODO: Pass from vehicle parameters
            let v_max = 36;
            pui.hline(egui::plot::HLine::new(v_max).style(egui::plot::LineStyle::Dashed { length: 10.0 })); // .name("v_max"));

            if let Some(pointer) = pui.pointer_coordinate() {
                cursor_x = Some(pointer.x);
            }
        });

    ui.heading("Acceleration");
    let _acceleration_plot = plot("acceleration_plot")
        .center_y_axis(true)
        .label_formatter(unit_label_formatter("m/s^2"))
        .show(ui, |pui| {
            pui.line(plot_data.acceleration);
            pui.line(plot_data.acceleration_ref);

            pui.vline(ts_vline.clone());

            // TODO: Pass from vehicle parameters
            let a_max = 2.5;
            pui.hline(egui::plot::HLine::new(a_max).style(egui::plot::LineStyle::Dashed { length: 10.0 })); //.name("maximum acceleration"));
            pui.hline(egui::plot::HLine::new(-a_max).style(egui::plot::LineStyle::Dashed { length: 10.0 })); //.name("minimum acceleration"));

            if let Some(pointer) = pui.pointer_coordinate() {
                cursor_x = Some(pointer.x);
            }
        });

    ui.heading("Orientation");
    let _theta_plot = plot("theta_plot")
        .center_y_axis(true)
        .label_formatter(|name, value| {
            if !name.is_empty() {
                let degs = value.y * std::f64::consts::FRAC_1_PI * 180.0;
                format!("{}:\n{:.*} rad ({:.*}°)", name, 2, value.y, 0, degs)
            } else {
                "".to_owned()
            }
        })
        .show(ui, |pui| {
            pui.line(plot_data.orientation);
            pui.line(plot_data.orientation_ref);

            pui.vline(ts_vline.clone());

            if let Some(pointer) = pui.pointer_coordinate() {
                cursor_x = Some(pointer.x);
            }
        });

    cursor_x
}

pub fn update_selected_color(
    mut trajectory_q: Query<(&TrajectoryLog, &PickSelection, &mut Stroke), Changed<PickSelection>>,
) {
    for (traj, selected, mut stroke) in trajectory_q.iter_mut() {
        if selected.is_selected {
            stroke.color = traj.selected_color();
        } else {
            stroke.color = traj.color();
        }
    }
}


pub fn trajectory_window(
    mut commands: Commands,

    mut contexts: EguiContexts,

    mut trajectory_q: Query<(Entity, &TrajectoryLog, &PickSelection)>,

    cts: Res<CurrentTimeStep>,

    mtraj: Res<MainTrajectory>,

    mut cached_plot_data: Local<Option<CachedTrajectoryPlotData>>,
) {
    let ctx = contexts.ctx_mut();

    let mut selected_traj = Option::None;
    for (entity, traj, selected) in trajectory_q.iter_mut() {
        if selected.is_selected {
            selected_traj = Some((entity, traj));
        }
    }

    let panel_id = egui::Id::new("side panel trajectory right");
    egui::SidePanel::right(panel_id)
        .exact_width(500.0)
        .show(ctx, |ui| {
            let Some((entity, traj)) = selected_traj else {
                *cached_plot_data = None;
                return;
            };

            let cplot_data = match cached_plot_data.as_ref() {
                Some(data) => {
                    if data.time_step == traj.time_step && data.trajectory_number == traj.trajectory_number && data.unique_id == traj.unique_id {
                        data.clone()
                    } else {
                        let cplot_data = CachedTrajectoryPlotData::from_trajectory(&mtraj, &traj);
                        *cached_plot_data = Some(cplot_data.clone());
                        cplot_data
                    }
                },
                None => {
                    let cplot_data = CachedTrajectoryPlotData::from_trajectory(&mtraj, &traj);
                    *cached_plot_data = Some(cplot_data.clone());
                    cplot_data
                },
            };

            ui.heading(format!("Trajectory {}", traj.trajectory_number));
            // ui.label(format!("type: {:#?}", obs.obstacle_type()));

            ui.label(format!("feasible: {}", traj.feasible));

            ui.label(format!("total cost: {}", traj.costs_cumulative_weighted));
            ui.label(format!("collision cost: {}", traj.costs.prediction_cost));
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

            let plot_data = TrajectoryPlotData::from_data(cplot_data);

            plot_traj(plot_data, ui, cts.dynamic_time_step.round());

            return;

            commands.entity(entity).despawn_descendants();
            match plot_traj(plot_data, ui, cts.dynamic_time_step.round()) {
                None => { },
                Some(ts) => {
                    let mut ts = ts.round() as i32;


                    if ts >= traj.time_step {
                        ts -= traj.time_step;
                    } else {
                        return;
                    }

                    let Some(pos) = traj
                        .kinematic_data
                        .positions()
                        .nth(ts as usize)
                        else { return; };

                    let pointer_shape = bevy_prototype_lyon::shapes::Circle {
                        radius: 1e3,
                        center: Vec2::ZERO,
                    };

                    commands.entity(entity).with_children(|builder| {
                        builder.spawn((
                            Name::new("pointer trajectory"),
                            PointerTimeStep::default(),
                            ShapeBundle {
                                path: GeometryBuilder::build_as(&pointer_shape),
                                transform: Transform::from_translation(
                                    pos.extend(100.0)
                                ).with_scale(Vec3::splat(1e-4)),
                                ..default()
                            },
                            Fill::color(Color::ORANGE_RED.with_a(0.6))
                        ));
                    });
                }
            };



        });
}

use std::collections::BTreeMap;

use bevy::prelude::*;

use bevy_mod_picking::prelude::*;
use bevy_prototype_lyon::prelude::*;

use bevy_egui::EguiContexts;

use crate::global_settings::CurrentTimeStep;

mod plot;
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

    s.split(',')
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
    kappa_rad: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    curvilinear_orientations_rad: Vec<f64>,
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

    fn make_plot_data(data: &[f64], shift: Option<i32>) -> Vec<[f64; 2]> {
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

    fn kappa_plot_data(&self, shift: Option<i32>) -> Vec<[f64; 2]> {
        Self::make_plot_data(&self.kappa_rad, shift)
    }

    fn curvilinear_orientation_plot_data(&self, shift: Option<i32>) -> Vec<[f64; 2]> {
        Self::make_plot_data(&self.curvilinear_orientations_rad, shift)
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
                40,    //100_u8.saturating_sub((traj.time_step as u8)), //.saturating_mul(4)),
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
        self.kinematic_data
            .acceleration_plot_data(Some(self.time_step))
    }

    fn orientation_plot_data(&self) -> Vec<[f64; 2]> {
        self.kinematic_data
            .orientation_plot_data(Some(self.time_step))
    }

    fn kappa_plot_data(&self) -> Vec<[f64; 2]> {
        self.kinematic_data.kappa_plot_data(Some(self.time_step))
    }

    fn curvilinear_orientation_plot_data(&self) -> Vec<[f64; 2]> {
        self.kinematic_data
            .curvilinear_orientation_plot_data(Some(self.time_step))
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

    let map = unsafe { memmap2::MmapOptions::new().populate().map(&file)? };
    let cursor = std::io::Cursor::new(map);

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(cursor);

    let res = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;

    Ok(res)
}

pub fn read_main_log(path: &std::path::Path) -> Result<Vec<MainLog>, Box<dyn std::error::Error>> {
    let mut rdr = csv::ReaderBuilder::new().delimiter(b';').from_path(path)?;

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

fn reassemble_main_trajectory(mtraj: &[MainLog]) -> KinematicData {
    let x_positions_m = mtraj
        .iter()
        .map(|traj| traj.x_position_vehicle_m)
        .collect::<Vec<f64>>();
    let y_positions_m = mtraj
        .iter()
        .map(|traj| traj.y_position_vehicle_m)
        .collect::<Vec<f64>>();

    let velocities_mps = mtraj
        .iter()
        .map(|traj| traj.kinematic_data.velocities_mps.first().copied())
        .collect::<Option<Vec<f64>>>()
        .unwrap();

    let accelerations_mps2 = mtraj
        .iter()
        .map(|traj| traj.kinematic_data.accelerations_mps2.first().copied())
        .collect::<Option<Vec<f64>>>()
        .unwrap();

    let theta_orientations_rad = mtraj
        .iter()
        .map(|traj| traj.kinematic_data.theta_orientations_rad.first().copied())
        .collect::<Option<Vec<f64>>>()
        .unwrap();

    let kappa_rad = mtraj
        .iter()
        .map(|traj| traj.kinematic_data.kappa_rad.first().copied())
        .collect::<Option<Vec<f64>>>()
        .unwrap();

    let curvilinear_orientations_rad = mtraj
        .iter()
        .map(|traj| {
            traj.kinematic_data
                .curvilinear_orientations_rad
                .first()
                .copied()
        })
        .collect::<Option<Vec<f64>>>()
        .unwrap();

    KinematicData {
        x_positions_m,
        y_positions_m,
        theta_orientations_rad,
        kappa_rad,
        curvilinear_orientations_rad,
        velocities_mps,
        accelerations_mps2,
    }
}

#[derive(Default, Component)]
pub struct PointerTimeStep {
    time_step: Option<f64>,
}

#[derive(Component, Reflect)]
pub(crate) struct TrajectoryGroup {
    time_step: i32,
}

fn make_trajectory_bundle(traj: &TrajectoryLog) -> Option<impl Bundle> {
    let points: Vec<Vec2> = traj.kinematic_data.positions().collect();

    if !points.iter().all(|v| v.x.is_finite() && v.y.is_finite()) {
        return None;
    }

    let traj_shape: shapes::Polygon = bevy_prototype_lyon::shapes::Polygon {
        points,
        closed: false,
    };

    Some((
        Name::new(format!("trajectory {}", traj.trajectory_number)),
        traj.to_owned(),
        ShapeBundle {
            path: GeometryBuilder::build_as(&traj_shape),
            transform: Transform::from_xyz(0.0, 0.0, 4.0 + (traj.unique_id as f32) * 1e-6),
            ..default()
        },
        Stroke::new(traj.color(), 0.05),
        On::<Pointer<Over>>::target_commands_mut(|_click, commands| {
            commands.insert(HoveredTrajectory);
        }),
        On::<Pointer<Out>>::target_commands_mut(|_click, commands| {
            commands.remove::<HoveredTrajectory>();
        }),
        PickableBundle::default(),
        RaycastPickTarget::default(),
    ))
}

fn make_main_trajectory_bundle(main_trajectories: &Vec<MainLog>) -> (MainTrajectory, impl Bundle) {
    let mpoints = main_trajectories
        .iter()
        .map(|traj| traj.kinematic_data.positions().next())
        .collect::<Option<Vec<Vec2>>>()
        .unwrap();

    let traj_shape: shapes::Polygon = bevy_prototype_lyon::shapes::Polygon {
        points: mpoints.clone(),
        closed: false,
    };

    let mtraj = MainTrajectory {
        path: mpoints,
        kinematic_data: reassemble_main_trajectory(&main_trajectories),
    };

    (
        mtraj,
        (
            Name::new("main trajectory"),
            ShapeBundle {
                path: GeometryBuilder::build_as(&traj_shape),
                transform: Transform::from_xyz(0.0, 0.0, 0.5),
                ..default()
            },
            Stroke::new(Color::rgba(0.4, 0.6, 0.18, 0.7), 0.15),
        ),
    )
}

pub fn spawn_trajectories(mut commands: Commands, args: Res<crate::args::Args>) {
    let main_trajectories_path = std::path::Path::join(&args.logs, "logs.csv");
    let main_trajectories =
        read_main_log(&main_trajectories_path).expect("could not read trajectory logs");
    let (mtraj_res, mtraj_bundle) = make_main_trajectory_bundle(&main_trajectories);

    commands.insert_resource(mtraj_res);

    commands.spawn(mtraj_bundle);

    let trajectories_path = std::path::Path::join(&args.logs, "trajectories.csv");
    // let io_pool = bevy::tasks::AsyncComputeTaskPool::get();
    let io_pool = bevy::tasks::TaskPoolBuilder::new().num_threads(8).build();

    let file = std::fs::File::open(trajectories_path).unwrap();

    let map = unsafe { memmap2::MmapOptions::new().populate().map(&file) }.unwrap();
    let cursor = std::io::Cursor::new(map);

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(cursor);

    // let mut sr = csv::StringRecord::default();

    let headers = rdr.headers().unwrap().to_owned();

    // let rdr = Arc::new(rdr);

    let (sender, receiver) = std::sync::mpsc::channel();
    let buf = std::sync::Arc::new(thingbuf::ThingBuf::<csv::StringRecord>::new(64));

    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let reader_task = io_pool.spawn({
        let buf = buf.clone();
        let done = done.clone();

        async move {
            while !rdr.is_done() {
                match buf.push_ref() {
                    Ok(mut bref) => {
                        rdr.read_record(&mut bref).unwrap();
                    }
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_micros(100));
                        continue;
                    }
                };
            }
            done.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    });

    reader_task.detach();
    for _ in 0..6 {
        //let done = done.
        let done = done.clone();
        let buf = buf.clone();
        let headers = headers.clone();
        let sender = sender.clone();

        let task: bevy::tasks::Task<Result<(), Box<dyn std::error::Error + Send>>> =
            io_pool.spawn({
                async move {
                    while !done.load(std::sync::atomic::Ordering::Relaxed) {
                        let Some(next) = buf.pop_ref() else {
                        std::thread::yield_now();
                        continue;
                    };

                        let tl: TrajectoryLog = next.deserialize(Some(&headers)).unwrap(); //map_err(Box::new).map_err(std::sync::Arc::new)?;
                        if let Some(bundle) = make_trajectory_bundle(&tl) {
                            sender.send((tl.time_step, bundle)).unwrap();
                        }

                        // bevy::log::info!("len={}", buf.len());
                    }
                    drop(sender);
                    Ok(())
                }
            });
        task.detach();
    }

    /*
    while rdr.read_record(&mut sr).unwrap() {
        let task = io_pool.spawn({
            let sr = sr.clone();
            let headers = headers.clone();
            let sender = sender.clone();
            async move {
                let tl: TrajectoryLog = sr.deserialize(Some(&headers)).unwrap();
                if let Some(bundle) = make_trajectory_bundle(&tl) {
                    sender.send((tl.time_step, bundle)).unwrap();
                }
                drop(sender);
                ()
            }
        });
        task.detach();
        // let res = futures_lite::future::block_on(task);
        // dbg!(res);
    }
    */

    drop(sender);

    let inserter = io_pool.scope(|s| {
        s.spawn({
            async move {
                let mut ts_map = BTreeMap::new();
                for (ts, bundle) in receiver.iter() {
                    //bevy::log::info!("received one");
                    let ts_entity = ts_map.entry(ts).or_insert_with(|| {
                        commands
                            .spawn((
                                Name::new(format!("trajectory group ts={}", ts)),
                                TrajectoryGroup { time_step: ts },
                                SpatialBundle::default(),
                            ))
                            .id()
                    });
                    commands.spawn(bundle).set_parent(*ts_entity);
                }
                bevy::log::info!("done");
            }
        });
    });

    // let res = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;
    /*
    let pool = bevy::tasks::AsyncComputeTaskPool::get();

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
            let Some(bundle) = make_trajectory_bundle(&traj) else {
                bevy::log::warn!("skipping invalid trajectory");
                continue;
            };
            commands.spawn(bundle).set_parent(ts_entity);
        }
    }
    */
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
        }
        Some(_ts) => {
            *visibility = Visibility::Visible;
        }
    }
}

pub fn trajectory_tooltip(
    mut contexts: EguiContexts,

    trajectory_q: Query<&TrajectoryLog, With<HoveredTrajectory>>,
) {
    let ctx = contexts.ctx_mut();

    let base_id = egui::Id::new("traj tooltip");

    if trajectory_q.is_empty() {
        return;
    }

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

    mut cached_plot_data: Local<Option<plot::CachedTrajectoryPlotData>>,
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                let Some((entity, traj)) = selected_traj else {
                *cached_plot_data = None;
                return;
            };

                let cplot_data = match cached_plot_data.as_ref() {
                    Some(data) => {
                        if data.time_step == traj.time_step
                            && data.trajectory_number == traj.trajectory_number
                            && data.unique_id == traj.unique_id
                        {
                            data.clone()
                        } else {
                            let cplot_data =
                                plot::CachedTrajectoryPlotData::from_trajectory(&mtraj, traj);
                            *cached_plot_data = Some(cplot_data.clone());
                            cplot_data
                        }
                    }
                    None => {
                        let cplot_data =
                            plot::CachedTrajectoryPlotData::from_trajectory(&mtraj, traj);
                        *cached_plot_data = Some(cplot_data.clone());
                        cplot_data
                    }
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

                let plot_data = plot::TrajectoryPlotData::from_data(cplot_data);

                plot::plot_traj(plot_data, ui, cts.dynamic_time_step.round());

                return;

                commands.entity(entity).despawn_descendants();
                match plot::plot_traj(plot_data, ui, cts.dynamic_time_step.round()) {
                    None => {}
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
                                    transform: Transform::from_translation(pos.extend(100.0))
                                        .with_scale(Vec3::splat(1e-4)),
                                    ..default()
                                },
                                Fill::color(Color::ORANGE_RED.with_a(0.6)),
                            ));
                        });
                    }
                };
            });
        });
}

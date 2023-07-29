use std::collections::BTreeMap;

use bevy::prelude::*;

use bevy_mod_picking::prelude::*;
use bevy_prototype_lyon::prelude::*;

use bevy_egui::EguiContexts;

use crate::global_settings::CurrentTimeStep;

mod plot;

pub(crate) mod log;

pub(crate) use log::{TrajectoryLog, MainLog, KinematicData};


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

    let selected_color = traj.selected_color();
    let normal_color = traj.color();

    Some((
        Name::new(format!("trajectory {}", traj.trajectory_number)),
        traj.to_owned(),
        ShapeBundle {
            path: GeometryBuilder::build_as(&traj_shape),
            transform: Transform::from_xyz(0.0, 0.0, 4.0 + (traj.unique_id as f32) * 1e-6),
            ..default()
        },
        {
            let mut stroke = Stroke::new(traj.color(), 0.02);
            stroke.options.tolerance = 10.0;
            stroke
        },

        On::<Pointer<Select>>::target_insert(Stroke::new(selected_color, 0.02)),
        On::<Pointer<Deselect>>::target_insert(Stroke::new(normal_color, 0.02)),

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


fn make_main_trajectory_bundle(main_trajectories: &[MainLog]) -> (MainTrajectory, impl Bundle) {
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
        kinematic_data: reassemble_main_trajectory(main_trajectories),
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
        log::read_main_log(&main_trajectories_path).expect("could not read trajectory logs");
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

    time_step: Res<crate::global_settings::TimeStep>,
) {
    if !time_step.is_changed() {
        return;
    }
    let time_step = time_step.time_step;
    bevy::log::info!("updating group visibility");

    for (traj, mut visibility) in trajectory_q.iter_mut() {
        if traj.time_step == time_step {
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

pub(crate) fn trajectory_visibility(
    mut trajectory_q: Query<(&TrajectoryLog, &mut Visibility)>,

    settings: Res<crate::global_settings::GlobalSettings>,
) {
    if !settings.is_changed() {
        return;
    }

    bevy::log::info!("updating traj visibility");

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

pub(crate) fn trajectory_tooltip(
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
                // ui.label(format!("collision cost: {}", traj.costs.prediction_cost));
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

fn trajectory_description(ui: &mut bevy_egui::egui::Ui, traj: &TrajectoryLog, plot_data: plot::TrajectoryPlotData, time_step: f32) {
    ui.label(egui::RichText::new(format!("Trajectory {}", traj.trajectory_number)).heading().size(30.0));
    // ui.label(format!("type: {:#?}", obs.obstacle_type()));

    ui.heading("Overview");
    ui.label(format!("feasible: {}", traj.feasible));

    let resp = ui.label(format!("total cost: {:.3}", traj.costs_cumulative_weighted));
    resp.on_hover_text(traj.costs_cumulative_weighted.to_string());
    // ui.label(format!("collision cost: {}", traj.costs.prediction_cost));

    ui.separator();

    use egui_extras::{TableBuilder, Column};
    TableBuilder::new(ui)
        .cell_layout(egui::Layout {
            main_align: egui::Align::Max,
            ..default()
        })
        .striped(true)
        .column(Column::initial(250.0).resizable(true))
        .column(Column::exact(70.0))
        .header(25.0, |mut header| {
            header.col(|ui| {
                ui.heading("Cost name");
            });
            header.col(|ui| {
                ui.heading("Value");
            });
        })
        .body(|mut body| {
            for (k, v) in traj.sorted_nonzero_costs() {
                body.row(18.0, |mut row| {
                    row.col(|ui| {
                        ui.label(egui::RichText::new(k).monospace());
                    });
                    row.col(|ui| {
                        let layout = egui::Layout::left_to_right(egui::Align::Center)
                            .with_main_align(egui::Align::Max)
                            .with_main_justify(true);
                        ui.with_layout(layout, |ui| {
                            let resp = ui.label(format!("{:>7.3}", v));
                            resp.on_hover_text(v.to_string());
                        });
                    });
                });
            }
        });

    ui.separator();

    ui.group(|ui| {
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
    });

    ui.separator();

    plot::plot_traj(plot_data, ui, time_step);
}

pub(crate) fn trajectory_window(
    commands: Commands,

    mut contexts: EguiContexts,

    trajectory_q: Query<(Entity, &TrajectoryLog, &PickSelection)>,

    cts: Res<CurrentTimeStep>,

    mtraj: Res<MainTrajectory>,

    mut cached_plot_data: Local<Option<plot::CachedTrajectoryPlotData>>,
) {
    let ctx = contexts.ctx_mut();

    let mut selected_traj = Option::None;
    for (entity, traj, selected) in trajectory_q.iter() {
        if selected.is_selected {
            selected_traj = Some((entity, traj));
        }
    }

    let panel_id = egui::Id::new("side panel trajectory right");
    egui::SidePanel::right(panel_id)
        .exact_width(500.0)
        .show(ctx, |ui| {
            ui.set_max_width(440.0);
            egui::ScrollArea::vertical()
                .max_width(440.0)
                .show(ui, |ui| {
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

                let plot_data = plot::TrajectoryPlotData::from_data(cplot_data);

                trajectory_description(ui, traj, plot_data, cts.dynamic_time_step.round());

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

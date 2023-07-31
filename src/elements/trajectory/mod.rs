use std::collections::BTreeMap;

use bevy::prelude::*;

use bevy_mod_picking::prelude::*;
use bevy_prototype_lyon::prelude::*;

use bevy_egui::EguiContexts;

use crate::global_settings::CurrentTimeStep;

mod plot;

pub(crate) mod log;

pub(crate) use log::{KinematicData, MainLog, TrajectoryLog};

#[derive(Resource)]
pub struct MainTrajectory {
    path: Vec<Vec2>,
    kinematic_data: KinematicData,
}

#[derive(Component, Default, Copy, Clone)]
pub struct HoveredTrajectory;

#[derive(Default, Component)]
pub struct PointerTimeStep {
    time_step: Option<f64>,
}

#[derive(Component, Reflect)]
pub(crate) struct TrajectoryGroup {
    time_step: i32,
}

#[derive(Component, Reflect, Clone, Copy)]
pub(crate) struct SelectedTrajectory;

#[derive(Component, Reflect, Clone, Copy)]
pub(crate) struct HasInvalidData;

#[derive(Component, Reflect, Clone, Copy)]
pub(crate) struct CurrentTrajectoryGroup;

#[derive(Event)]
pub(crate) struct SelectTrajectoryEvent(Entity);

impl From<bevy_eventlistener::callbacks::ListenerInput<Pointer<Select>>> for SelectTrajectoryEvent {
    fn from(value: bevy_eventlistener::callbacks::ListenerInput<Pointer<Select>>) -> Self {
        Self(value.listener())
    }
}

fn make_trajectory_bundle(traj: &TrajectoryLog) -> Option<(impl Bundle, Option<impl Bundle>)> {
    let points: Vec<Vec2> = traj.kinematic_data.positions().collect();

    if !points.iter().all(|v| v.x.is_finite() && v.y.is_finite()) {
        return None;
    }

    let traj_shape: shapes::Polygon = bevy_prototype_lyon::shapes::Polygon {
        points,
        closed: false,
    };

    let base_bundle = (
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
        On::<Pointer<Select>>::send_event::<SelectTrajectoryEvent>(),
        // On::<Pointer<Select>>::target_insert((SelectedTrajectory, Stroke::new(selected_color, 0.02))),
        // On::<Pointer<Deselect>>::target_commands_mut(|_ptr, commands| {
            // commands.insert(Stroke::new(normal_color, 0.02));
            // commands.remove::<SelectedTrajectory>();
        //}),
        On::<Pointer<Over>>::target_insert(HoveredTrajectory),
        On::<Pointer<Out>>::target_remove::<HoveredTrajectory>(),
        PickableBundle::default(),
        RaycastPickTarget::default(),
    );

    if traj.costs.values().any(|v| !v.is_finite()) {
        Some((base_bundle, Some(HasInvalidData)))
    } else {
        Some((base_bundle, None))
    }
}

pub(super) fn update_selected_trajectory(
    mut commands: Commands,

    mut selection_events: EventReader<SelectTrajectoryEvent>,

    mut trajectory_q: Query<(&TrajectoryLog, &mut Transform, &mut Visibility), Without<SelectedTrajectory>>,

    mut selected_q: Query<(Entity, &TrajectoryLog, &mut Visibility), With<SelectedTrajectory>>,
) {
    let new_selection = !selection_events.is_empty();
    if !new_selection {
        return;
    }

    for (entity, traj, mut visibility) in selected_q.iter_mut() {
        let mut ecommands = commands.entity(entity);
        ecommands.remove::<SelectedTrajectory>();

        let normal_color = traj.color();
        ecommands.insert(Stroke::new(normal_color, 0.02));

        *visibility = if traj.feasible { Visibility::Inherited } else { Visibility::Hidden };
    }

    let SelectTrajectoryEvent(entity) = selection_events.iter().last().unwrap();
    bevy::log::debug!("handling selection for entity {:?}", entity);

    let mut ecommands = commands.entity(*entity);
    ecommands.insert(SelectedTrajectory);

    let Ok((selected, mut transform, mut visibility)) = trajectory_q.get_mut(*entity) else {
        return;
    };
    *visibility = Visibility::Visible;
    transform.translation.z += 10.0;
    let selected_color = selected.selected_color();
    ecommands.insert(Stroke::new(selected_color, 0.05));
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
        kinematic_data: log::reassemble_main_trajectory(main_trajectories),
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

        let task: bevy::tasks::Task<Result<(), Box<dyn std::error::Error + Send + Sync>>> = io_pool
            .spawn({
                async move {
                    while !done.load(std::sync::atomic::Ordering::Relaxed) {
                        let Some(next) = buf.pop_ref() else {
                            std::thread::yield_now();
                            continue;
                        };

                        let tl: TrajectoryLog = next.deserialize(Some(&headers))?; //map_err(Box::new).map_err(std::sync::Arc::new)?;
                        if let Some(bundle) = make_trajectory_bundle(&tl) {
                            sender.send((tl.time_step, bundle))?;
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

    let _inserter = io_pool.scope(|s| {
        s.spawn({
            async move {
                let mut ts_map = BTreeMap::new();
                for (ts, (bundle, extra_bundle)) in receiver.iter() {
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
                    let mut entity = commands.spawn(bundle);
                    entity.set_parent(*ts_entity);
                    if let Some(extra) = extra_bundle {
                        entity.insert(extra);
                    }
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
    mut commands: Commands,

    mut trajectory_q: Query<(Entity, &TrajectoryGroup, &mut Visibility)>,

    // trajectory_child_q: Query<&TrajectoryLog>,
    time_step: Res<crate::global_settings::TimeStep>,
) {
    if !time_step.is_changed() {
        return;
    }
    let time_step = time_step.time_step;
    bevy::log::debug!("updating group visibility");

    for (entity, traj, mut visibility) in trajectory_q.iter_mut() {
        if traj.time_step == time_step {
            *visibility = Visibility::Visible;
            commands.entity(entity).insert(CurrentTrajectoryGroup);
        } else {
            *visibility = Visibility::Hidden;
            commands.entity(entity).remove::<CurrentTrajectoryGroup>();
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

    bevy::log::debug!("updating traj visibility");

    for (traj, mut visibility) in trajectory_q.iter_mut() {
        if traj.feasible || settings.show_infeasible {
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[allow(unused)]
pub(crate) fn trajectory_cursor(
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

fn trajectory_description(
    ui: &mut bevy_egui::egui::Ui,
    traj: &TrajectoryLog,
    plot_data: plot::TrajectoryPlotData,
    time_step: f32,
    issues_detected: bool,
) -> (bool, Option<f64>) {
    let mut xcursor = None;

    let issues_detected = std::cell::RefCell::new(issues_detected);

    macro_rules! rich_label_base {
        ($val:ident, $fmt:expr) => {
            if $val.is_nan() {
                *issues_detected.borrow_mut() = true;
                egui::RichText::new("NaN").color(egui::Color32::LIGHT_RED)
            } else if $val.is_infinite() {
                *issues_detected.borrow_mut() = true;
                egui::RichText::new($val.to_string()).color(egui::Color32::LIGHT_RED)
            } else {
                egui::RichText::new(format!($fmt, $val)).monospace()
            }
        };
    }

    macro_rules! rich_label {
        ($ui:ident, $val:expr, $fmt:expr) => {
            let val: f64 = $val;
            let text: egui::RichText = rich_label_base!(val, $fmt);

            let resp = $ui.label(text);
            resp.on_hover_text(val.to_string());
        };
        ($ui:ident, $val:expr, $fmt:expr, $weak_th:expr) => {
            let val: f64 = $val;
            let base_text: egui::RichText = rich_label_base!(val, $fmt);
            let text = if val < $weak_th {
                base_text.weak()
            } else {
                base_text
            };

            let resp = $ui.label(text);
            resp.on_hover_text(val.to_string());
        };
    }

    let value_cell_layout = egui::Layout::left_to_right(egui::Align::Center)
        .with_main_align(egui::Align::Max)
        .with_main_justify(true);

    ui.horizontal_top(|ui| {
    ui.label(
        egui::RichText::new(format!(
            "Trajectory {} (id: {})",
            traj.trajectory_number, traj.unique_id
        ))
        .heading()
        .size(26.0),
    );

        if *issues_detected.borrow() {
            let resp = ui.label(egui::RichText::new("\u{26A0}").heading().size(24.0).color(egui::Color32::YELLOW));
            resp.on_hover_text("Some values (e.g. computed costs) for this trajectory are invalid because they are NaN or infinity");
        }
    });

    ui.heading("Overview");
    ui.horizontal_top(|ui| {
        ui.label("feasible:");
        ui.label(egui::RichText::new(traj.feasible.to_string()).strong());
    });

    ui.horizontal_top(|ui| {
        ui.label("total cost:");
        rich_label!(ui, traj.costs_cumulative_weighted, "{:.3}");
    });

    let overview_left = |ui: &mut egui::Ui| {
        ui.label("final position:");
        ui.indent("curvilinear position", |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label("longitudinal:");
                    ui.label("lateral:");
                });
                ui.vertical(|ui| {
                    rich_label!(ui, traj.s_position_m, "{:>8.3}");
                    rich_label!(ui, traj.d_position_m, "{:>8.3}");
                    //ui.label(format!("{:>8.2}", traj.s_position_m));
                    //ui.label(format!("{:>8.2}", traj.d_position_m));
                });
            });
        });

        match (traj.ego_risk, traj.obst_risk) {
            (Some(ego_risk), Some(obst_risk)) => {
                ui.horizontal_top(|ui| {
                    ui.label("ego risk:");
                    rich_label!(ui, ego_risk, "{:.3}");
                });
                ui.horizontal_top(|ui| {
                    ui.label("obst risk:");
                    rich_label!(ui, obst_risk, "{:.3}");
                });
            }
            _ => {}
        };
    };

    use egui_extras::{Column, TableBuilder};
    let overview_right = |ui: &mut egui::Ui| {
        ui.push_id("overview table", |ui| {
            TableBuilder::new(ui)
                .column(Column::auto())
                .column(Column::initial(50.0))
                .body(|mut body| {
                    body.row(15.0, |mut row| {
                        row.col(|ui| {
                            let resp = ui.label("\u{0394}t:");
                            resp.on_hover_text("time step size");
                        });
                        row.col(|ui| {
                            ui.with_layout(value_cell_layout, |ui| {
                                rich_label!(ui, traj.dt, "{}\u{2006}s");
                            });
                        });
                    });
                    body.row(15.0, |mut row| {
                        row.col(|ui| {
                            ui.label("horizon:");
                        });
                        row.col(|ui| {
                            ui.with_layout(value_cell_layout, |ui| {
                                rich_label!(ui, traj.horizon, "{}\u{2006}s");
                            });
                        });
                    });
                    body.row(15.0, |mut row| {
                        row.col(|ui| {
                            ui.label("trajectory length:");
                        });
                        row.col(|ui| {
                            ui.with_layout(value_cell_layout, |ui| {
                                rich_label!(ui, traj.actual_traj_length, "{} m");
                            });
                        });
                    });
                });
        });
    };
    egui_extras::StripBuilder::new(ui)
        .size(egui_extras::Size::initial(70.0))
        .size(egui_extras::Size::remainder())
        .vertical(|mut strip| {
            strip.strip(|builder| {
                builder
                    .size(egui_extras::Size::relative(0.5))
                    .size(egui_extras::Size::relative(0.5))
                    .horizontal(|mut strip| {
                        strip.cell(overview_left);
                        strip.cell(overview_right);
                    });
            });
            strip.cell(|ui| {
                // let resp = ui.label(format!("total cost: {:.3}", traj.costs_cumulative_weighted));
                //resp.on_hover_text(traj.costs_cumulative_weighted.to_string());

                ui.separator();

                ui.heading("Costs");

                let hide_small_costs_id = ui.make_persistent_id("hide small costs");
                let cost_threshold_id = ui.make_persistent_id("cost map threshold");

                let mut hide_small_costs =
                    ui.data_mut(|itm| *itm.get_persisted_mut_or::<bool>(hide_small_costs_id, true));
                let mut cost_threshold =
                    ui.data_mut(|itm| *itm.get_persisted_mut_or::<f64>(cost_threshold_id, 1e-3));

                ui.checkbox(&mut hide_small_costs, "Hide small cost functions");
                ui.add_enabled_ui(hide_small_costs, |ui| {
                    ui.indent("cost threshold section", |ui| {
                        ui.add(
                            egui::Slider::new(&mut cost_threshold, 1e-4..=1e1)
                                .logarithmic(true)
                                .text("cost display threshold"),
                        );
                    });
                });

                ui.data_mut(|itm| itm.insert_persisted(hide_small_costs_id, hide_small_costs));
                ui.data_mut(|itm| itm.insert_persisted(cost_threshold_id, cost_threshold));

                ui.add_space(5.0);

                ui.push_id("cost table", |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .column(Column::exact(300.0).resizable(true))
                        .column(Column::remainder())
                        .header(25.0, |mut header| {
                            header.col(|ui| {
                                ui.label(
                                    egui::RichText::new("Cost Function Name")
                                        .heading()
                                        .size(14.0),
                                );
                            });
                            header.col(|ui| {
                                ui.label(egui::RichText::new("Value").heading().size(14.0));
                            });
                        })
                        .body(|mut body| {
                            for (k, v) in traj.sorted_nonzero_costs(if hide_small_costs {
                                Some(cost_threshold)
                            } else {
                                None
                            }) {
                                body.row(18.0, |mut row| {
                                    row.col(|ui| {
                                        ui.monospace(k);
                                    });
                                    row.col(|ui| {
                                        ui.with_layout(value_cell_layout, |ui| {
                                            rich_label!(
                                                ui,
                                                v,
                                                "{:>7.3}",
                                                traj.costs_cumulative_weighted * 0.05
                                            );
                                        });
                                    });
                                });
                            }
                        });
                });

                ui.add_space(10.0);

                ui.heading("Feasability");

                ui.push_id("feasability table", |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .column(Column::exact(300.0).resizable(true))
                        .column(Column::remainder())
                        .header(25.0, |mut header| {
                            header.col(|ui| {
                                ui.label(egui::RichText::new("Check Name").heading().size(14.0));
                            });
                            header.col(|ui| {
                                let resp = ui.label(
                                    egui::RichText::new("Violated Count").heading().size(14.0),
                                );
                                resp.on_hover_text(
                        "Number of time steps in which this feasability check was violated",
                    );
                            });
                        })
                        .body(|mut body| {
                            macro_rules! inf_row {
                                ($inf_name:ident) => {
                                    body.row(18.0, |mut row| {
                                        row.col(|ui| {
                                            ui.monospace(std::stringify!($inf_name));
                                        });
                                        row.col(|ui| {
                                            ui.with_layout(value_cell_layout, |ui| {
                                                let text = egui::RichText::new(format!("{:>5.0}", traj.$inf_name)).monospace();
                                                let resp = ui.label(if traj.$inf_name == 0.0 { text.weak() } else { text });
                                                resp.on_hover_text(traj.$inf_name.to_string());
                                            });
                                        });
                                    });
                                };
                            }
                            inf_row!(inf_kin_yaw_rate);
                            inf_row!(inf_kin_acceleration);
                            inf_row!(inf_kin_max_curvature);
                        });
                });

                ui.separator();

                xcursor = plot::plot_traj(plot_data, ui, time_step)
            });
        });

    (issues_detected.take(), xcursor)
}

pub(crate) fn trajectory_window(
    mut commands: Commands,

    mut contexts: EguiContexts,

    trajectory_q: Query<(Entity, &TrajectoryLog, bevy::ecs::query::Has<HasInvalidData>), With<SelectedTrajectory>>,

    cts: Res<CurrentTimeStep>,

    mtraj: Res<MainTrajectory>,

    mut cached_plot_data: Local<Option<std::sync::Arc<plot::CachedTrajectoryPlotData>>>,
) {
    let ctx = contexts.ctx_mut();


    let panel_id = egui::Id::new("side panel trajectory right");
    egui::SidePanel::right(panel_id)
        .default_width(500.0)
        .min_width(350.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_width(500.0 - ctx.style().spacing.scroll_bar_width - 8.0)
                .scroll_bar_visibility(
                    egui::containers::scroll_area::ScrollBarVisibility::AlwaysVisible,
                )
                .show(ui, |ui| {
                    let Ok(selected_traj) = trajectory_q.get_single() else {
                        *cached_plot_data = None;
                        return;
                    };
                    let (entity, traj, invalid_data) = selected_traj;

                    // cached_plot_data.
                    // cached_plot_data.
                    let mut new_data = cached_plot_data
                        .take()
                        .filter(|data| data.matches_trajectory(traj));
                    let cplot_data1 = new_data
                        .get_or_insert_with(|| std::sync::Arc::new(plot::CachedTrajectoryPlotData::from_trajectory(&mtraj, traj)));

                    let plot_data = plot::TrajectoryPlotData::from_data(cplot_data1);

                    *cached_plot_data = new_data;

                    let (new_issues_detected, xcursor) = trajectory_description(
                        ui,
                        traj,
                        plot_data,
                        cts.dynamic_time_step.round(),
                        invalid_data,
                    );
                    if new_issues_detected && invalid_data != new_issues_detected {
                        commands.entity(entity).insert(HasInvalidData);
                    }

                    // TODO: Fix remaining trajectory cursor issues
                    let enable_trajectory_cursor = false;
                    if !enable_trajectory_cursor {
                        return;
                    }

                    commands.entity(entity).despawn_descendants();
                    match xcursor {
                        None => {}
                        Some(ts) => {
                            let mut ts = ts.round() as i32;

                            if ts >= traj.time_step {
                                ts -= traj.time_step;
                            } else {
                                return;
                            }

                            let Some(pos) = traj.kinematic_data.positions().nth(ts as usize) else {
                                return;
                            };

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


macro_rules! rich_label_base {
    ($val:ident, $fmt:expr) => {
        if $val.is_nan() {
            egui::RichText::new("NaN").color(egui::Color32::LIGHT_RED)
        } else if $val.is_infinite() {
            egui::RichText::new($val.to_string()).color(egui::Color32::LIGHT_RED)
        } else {
            egui::RichText::new(format!($fmt, $val)).monospace()
        }
    };
}

macro_rules! rich_label {
    ($ui:ident, $val:expr, $fmt:expr) => {
        let val: f64 = $val.into();
        let text: egui::RichText = rich_label_base!(val, $fmt);

        let resp = $ui.label(text);
        resp.on_hover_text(val.to_string());
    };
    ($ui:ident, $val:expr, $fmt:expr, $weak_th:expr) => {
        let val: f64 = $val.into();
        let base_text: egui::RichText = rich_label_base!(val, $fmt);
        let text = if val < $weak_th {
            base_text.weak()
        } else {
            base_text
        };

        let resp = $ui.label(text);
        resp.on_hover_text(val.to_string());
    };
}

#[derive(Default, PartialEq, Eq)]
pub(crate) enum TrajectorySortKey {
    #[default]
    ID,
    MaxCurvilinearDeviation,
    FinalVelocity,
    Cost,
}

#[derive(Default)]
pub(crate) enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    fn toggle(&mut self) {
        *self = self.reverse();
    }

    fn reverse(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }
}

use crate::finite::{Finite, Finite32};

impl TrajectoryLog {

    fn max_deviation(&self) -> Finite32 {
        self
            .kinematic_data
            .curvilinear_orientations_rad
            .iter()
            .map(|v| v.abs().try_into().unwrap())
            .max()
            // .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    fn final_velocity(&self) -> Finite32 {
        self
            .kinematic_data
            .velocities_mps
            .iter()
            .last()
            .copied()
            .unwrap()
            .try_into()
            .unwrap()
    }
}


pub(crate) fn trajectory_list(
    mut contexts: EguiContexts,

    selected_q: Query<Entity, Added<SelectedTrajectory>>,

    trajectory_q: Query<(&TrajectoryLog, bevy::ecs::query::Has<SelectedTrajectory>, bevy::ecs::query::Has<HasInvalidData>)>,

    mut group_q: Query<&mut Children, With<CurrentTrajectoryGroup>>,

    cts: Res<CurrentTimeStep>,

    mut send_selection_event: EventWriter<SelectTrajectoryEvent>,

    mut sort_key: Local<TrajectorySortKey>,
    mut sort_dir: Local<SortDirection>,
) {
    let ctx = contexts.ctx_mut();

    let Ok(mut children) = group_q.get_single_mut() else {
        return;
    };

    let key = |entity: &Entity| {
        let (traj, _selected, _invalid_data) = trajectory_q.get(*entity).unwrap();

        let val = match *sort_key {
            TrajectorySortKey::ID => {
                Finite::from(traj.unique_id)
            },
            TrajectorySortKey::MaxCurvilinearDeviation => {
                traj.max_deviation()
            },
            TrajectorySortKey::FinalVelocity => {
                traj.final_velocity()
            },
            TrajectorySortKey::Cost => {
                (traj.costs_cumulative_weighted as f32)
                    .try_into()
                    .unwrap_or(Finite::MAX)
            },
        };
        match *sort_dir {
            SortDirection::Ascending => {
                val
            },
            SortDirection::Descending => {
                -val
            },
        }
    };
    children.sort_by_cached_key(key);

    let selected_idx = selected_q
        .get_single()
        .ok()
        .and_then(|selected| {
            children
                .iter()
                .position(|entity| *entity == selected)
        });

    let value_cell_layout = egui::Layout::left_to_right(egui::Align::Center)
        .with_main_align(egui::Align::Max)
        .with_main_justify(true);

    use egui_extras::{Column, TableBuilder};
    egui::Window::new(format!("Trajectory List for Time Step {}", cts.fixed_time_step()))
        .id(egui::Id::new("trajectory list window"))
        .show(ctx, |ui| {
        let mut tb = TableBuilder::new(ui)
            .striped(true)
            .column(Column::initial(120.0).resizable(true))
            .column(Column::initial(100.0).resizable(true).clip(true))
            .column(Column::initial(100.0).resizable(true).clip(true))
            .column(Column::exact(100.0).clip(true))
            ;

        if let Some(idx) = selected_idx {
            tb = tb.scroll_to_row(idx, None);
        }
        tb.header(34.0, |mut header| {
                macro_rules! header_entry {
                    ($key:expr, $label:expr) => {
                        header.col(|ui| {
                            use core::ops::DerefMut;
                            let selected = *sort_key == $key;
                            let sort_dir_symbol = match *sort_dir {
                                SortDirection::Ascending => '\u{2B06}',
                                SortDirection::Descending => '\u{2B07}',
                            };
                            let label_text = if selected { format!("{} {}", $label, sort_dir_symbol) } else { $label.to_string() };
                            let resp = ui.selectable_value(sort_key.deref_mut(), $key, egui::RichText::new(label_text).heading().size(14.0));
                            if resp.changed() {
                                *sort_dir = SortDirection::default();
                            }
                            if resp.clicked() && selected {
                                sort_dir.toggle();
                            };
                        })
                    }
                }

                header_entry!(TrajectorySortKey::ID, "Trajectory ID");
                let (_rect, resp) = header_entry!(TrajectorySortKey::MaxCurvilinearDeviation, "Max Curv \u{0394}");
                resp.on_hover_text("Maximum Absolute Curvilinear Deviation/Relative Orientation");
                header_entry!(TrajectorySortKey::FinalVelocity, "Final Velocity");
                header_entry!(TrajectorySortKey::Cost, "Cost");
            })
            .body(|body| {
                body.rows(18.0, children.len(), |row_index, mut row| {
                    let entity = *children.get(row_index).unwrap();
                    let (traj, selected, invalid_data) = trajectory_q.get(entity).unwrap();
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            let clicked = ui.selectable_label(selected, traj.unique_id.to_string()).clicked();
                            if !selected && clicked {
                                send_selection_event.send(SelectTrajectoryEvent(entity));
                            }

                            if invalid_data {
                                let resp = ui.label(egui::RichText::new("\u{26A0}").color(egui::Color32::YELLOW));
                                resp.on_hover_text("Some values (e.g. computed costs) for this trajectory are invalid because they are NaN or infinity");
                            }

                            if !traj.feasible {
                                ui.label(egui::RichText::new("(infeasible)").italics().weak());
                            }
                        });
                    });
                    row.col(|ui| {
                        let max_deviation: f32 = traj.max_deviation().into();

                        ui.with_layout(value_cell_layout, |ui| {
                            rich_label!(ui, max_deviation, "{:>10.4} rad");
                        });
                    });
                    row.col(|ui| {
                        let final_velocity: f32 = traj.final_velocity().into();

                        ui.with_layout(value_cell_layout, |ui| {
                            rich_label!(ui, final_velocity, "{:>10.2} m/s");
                        });
                    });
                    row.col(|ui| {
                        ui.with_layout(value_cell_layout, |ui| {
                            rich_label!(ui, traj.costs_cumulative_weighted, "{:>8.3}");
                        });
                    });
                });
            });
    });
}

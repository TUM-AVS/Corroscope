use bevy::prelude::*;

use egui_plot::{Legend, PlotPoint};

pub(crate) struct TrajectoryPlotData {
    velocity: egui_plot::Line,
    velocity_ref: egui_plot::Line,
    acceleration: egui_plot::Line,
    acceleration_ref: egui_plot::Line,
    orientation: egui_plot::Line,
    orientation_ref: egui_plot::Line,
    curvilinear_orientation: egui_plot::Line,
    curvilinear_orientation_ref: egui_plot::Line,
    kappa: egui_plot::Line,
    kappa_ref: egui_plot::Line,
    trajectory_long: egui_plot::Line,
    trajectory_lat: egui_plot::Line,
    initial_velocity: f32,
}

#[derive(Resource, Clone)]
pub struct CachedTrajectoryPlotData {
    pub(crate) time_step: i32,
    pub(crate) trajectory_number: i32,
    pub(crate) unique_id: i32,
    velocity: Vec<[f64; 2]>,
    velocity_ref: Vec<[f64; 2]>,
    acceleration: Vec<[f64; 2]>,
    acceleration_ref: Vec<[f64; 2]>,
    orientation: Vec<[f64; 2]>,
    orientation_ref: Vec<[f64; 2]>,
    curvilinear_orientation: Vec<[f64; 2]>,
    curvilinear_orientation_ref: Vec<[f64; 2]>,
    kappa: Vec<[f64; 2]>,
    kappa_ref: Vec<[f64; 2]>,
    trajectory_long: Vec<[f64; 2]>,
    trajectory_lat: Vec<[f64; 2]>,
}

impl CachedTrajectoryPlotData {
    pub(crate) fn matches_trajectory(&self, traj: &super::TrajectoryLog) -> bool {
        self.time_step == traj.time_step
            && self.trajectory_number == traj.trajectory_number
            && self.unique_id == traj.unique_id
    }

    pub(crate) fn from_trajectory(
        mtraj: &super::MainTrajectory,
        traj: &super::TrajectoryLog,
    ) -> Self {
        let velocity = traj.velocity_plot_data();
        let velocity_ref = mtraj.kinematic_data.velocity_plot_data(None);

        let acceleration = traj.acceleration_plot_data();
        let acceleration_ref = mtraj.kinematic_data.acceleration_plot_data(None);

        let orientation = traj.orientation_plot_data();
        let orientation_ref = mtraj.kinematic_data.orientation_plot_data(None);

        let curvilinear_orientation = traj.curvilinear_orientation_plot_data();
        let curvilinear_orientation_ref =
            mtraj.kinematic_data.curvilinear_orientation_plot_data(None);

        let kappa = traj.kappa_plot_data();
        let kappa_ref = mtraj.kinematic_data.kappa_plot_data(None);

        let trajectory_long: Vec<[f64; 2]> = traj.trajectory_long_plot_data();
        let trajectory_lat: Vec<[f64; 2]> = traj.trajectory_lat_plot_data();

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
            curvilinear_orientation,
            curvilinear_orientation_ref,
            kappa,
            kappa_ref,
            trajectory_long,
            trajectory_lat,
        }
    }
}

impl TrajectoryPlotData {
    pub(crate) fn from_data(plot_data: &CachedTrajectoryPlotData) -> Self {
        let velocity = egui_plot::Line::new(plot_data.velocity.clone()).name("velocity [m/s]");

        let velocity_ref =
            egui_plot::Line::new(plot_data.velocity_ref.clone()).name("reference velocity [m/s]");

        let acceleration = egui_plot::Line::new(plot_data.acceleration.clone())
            .name("acceleration [m/s\u{00B2}]");

        let acceleration_ref = egui_plot::Line::new(plot_data.acceleration_ref.clone())
            .name("reference acceleration [m/s\u{00B2}]");

        let orientation =
            egui_plot::Line::new(plot_data.orientation.clone()).name("global orientation [rad]");

        let orientation_ref = egui_plot::Line::new(plot_data.orientation_ref.clone())
            .style(egui_plot::LineStyle::Dotted { spacing: 6.0 })
            .name("reference global orientation [rad]");

        let curvilinear_orientation =
            egui_plot::Line::new(plot_data.curvilinear_orientation.clone())
                .name("curvilinear orientation [rad]");

        let curvilinear_orientation_ref =
            egui_plot::Line::new(plot_data.curvilinear_orientation_ref.clone())
                .style(egui_plot::LineStyle::Dotted { spacing: 6.0 })
                .name("reference curvilinear orientation [rad]");

        let kappa = egui_plot::Line::new(plot_data.kappa.clone()).name("curvature [1/m]");

        let kappa_ref = egui_plot::Line::new(plot_data.kappa_ref.clone())
            .style(egui_plot::LineStyle::Dotted { spacing: 6.0 })
            .name("reference curvature [1/m]");

        let initial_velocity = plot_data.velocity.first().unwrap()[1] as f32;

        let trajectory_long = egui_plot::Line::new(plot_data.trajectory_long.clone()).name("longitudinal");
        let trajectory_lat = egui_plot::Line::new(plot_data.trajectory_lat.clone()).name("lateral");

        Self {
            velocity,
            velocity_ref,
            acceleration,
            acceleration_ref,
            orientation,
            orientation_ref,
            curvilinear_orientation,
            curvilinear_orientation_ref,
            kappa,
            kappa_ref,
            initial_velocity,
            trajectory_long,
            trajectory_lat
        }
    }
}

pub(crate) fn plot_traj(
    plot_data: TrajectoryPlotData,
    ui: &mut egui::Ui,
    time_step: f32,
    vparams: &super::VehicleParams,
) -> Option<f64> {
    let mut cursor_x = None;

    let group = egui::Id::new("trajectory plot group");

    let plot_width = ui.available_width();

    let base_plot = |name: &'static str| {
        egui_plot::Plot::new(name)
            // .legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftBottom))
            // .allow_drag(false)
            // .allow_zoom(false)
            .view_aspect(2.0)
            .min_size(egui::Vec2::new(150.0, 75.0))
            .sharp_grid_lines(true)
            // .include_x(0.0)
            // .include_y(0.0)
            .width(plot_width)
            .link_cursor(group, true, false)
    };

    let plot = |name: &'static str| {
        egui_plot::Plot::new(name)
            .legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftBottom))
            // .allow_drag(false)
            // .allow_zoom(false)
            .view_aspect(2.0)
            .min_size(egui::Vec2::new(150.0, 75.0))
            .sharp_grid_lines(true)
            .include_x(0.0)
            .include_y(0.0)
            .width(plot_width)
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

    let ts_vline = egui_plot::VLine::new(time_step)
        // .name("current time step")
        .style(egui_plot::LineStyle::Dotted { spacing: 0.1 });

    ui.heading("Longitudinal");
    let _velocity_plot: egui_plot::PlotResponse<()> = base_plot("long_plot")
        .y_grid_spacer(egui_plot::uniform_grid_spacer(|_grid_input| {
            [10.0, 2.0, 0.5]
        }))
        // .label_formatter(unit_label_formatter("m/s"))
        .show(ui, |pui| {
            pui.line(plot_data.trajectory_long);

            // pui.line()

            pui.vline(ts_vline.clone());

            if let Some(pointer) = pui.pointer_coordinate() {
                cursor_x = Some(pointer.x);
            }
        });
    ui.heading("Lateral");
    let _velocity_plot: egui_plot::PlotResponse<()> = base_plot("lat_plot")
        .y_grid_spacer(egui_plot::uniform_grid_spacer(|_grid_input| {
            [10.0, 2.0, 0.5]
        }))
        // .label_formatter(unit_label_formatter("m/s"))
        .show(ui, |pui| {
            pui.line(plot_data.trajectory_lat);

            pui.vline(ts_vline.clone());

            if let Some(pointer) = pui.pointer_coordinate() {
                cursor_x = Some(pointer.x);
            }
        });
    

    ui.heading("Velocity");
    let _velocity_plot = plot("velocity_plot")
        .y_grid_spacer(egui_plot::uniform_grid_spacer(|_grid_input| {
            [10.0, 2.0, 0.5]
        }))
        .label_formatter(unit_label_formatter("m/s"))
        .show(ui, |pui| {
            pui.line(plot_data.velocity);
            pui.line(plot_data.velocity_ref);

            pui.vline(ts_vline.clone());

            // pui.hline(
            //     egui_plot::HLine::new(vparams.v_max).style(egui_plot::LineStyle::Dashed { length: 10.0 }),
            // ); // .name("v_max"));

            if let Some(pointer) = pui.pointer_coordinate() {
                cursor_x = Some(pointer.x);
            }
        });

    ui.heading("Acceleration");

    ui.label(egui::RichText::new(
        "Note: Acceleration limits are calculated using switching velocity and initial velocity"
    ).weak());

    let _acceleration_plot = plot("acceleration_plot")
        .center_y_axis(true)
        .label_formatter(unit_label_formatter("m/s^2"))
        .show(ui, |pui| {
            pui.line(plot_data.acceleration);
            pui.line(plot_data.acceleration_ref);

            pui.vline(ts_vline.clone());


            let a_max = if plot_data.initial_velocity > vparams.v_switch  {
                vparams.a_max * vparams.v_switch / plot_data.initial_velocity
            } else {
                vparams.a_max
            };

            pui.hline(
                egui_plot::HLine::new(a_max).style(egui_plot::LineStyle::Dashed { length: 10.0 }),
            ); //.name("maximum acceleration"));
            pui.hline(
                egui_plot::HLine::new(-vparams.a_max)
                    .style(egui_plot::LineStyle::Dashed { length: 10.0 }),
            ); //.name("minimum acceleration"));

            if let Some(pointer) = pui.pointer_coordinate() {
                cursor_x = Some(pointer.x);
            }
        });

    let angle_label_formatter = |name: &str, value: &PlotPoint| {
        if !name.is_empty() {
            let degs = value.y * std::f64::consts::FRAC_1_PI * 180.0;
            format!("{}:\n{:.*} rad ({:.*}°)", name, 2, value.y, 0, degs)
        } else {
            "".to_owned()
        }
    };

    ui.heading("Global Orientation");
    let _theta_plot = plot("theta_plot")
        .center_y_axis(true)
        .label_formatter(angle_label_formatter)
        .show(ui, |pui| {
            pui.line(plot_data.orientation);
            pui.line(plot_data.orientation_ref);

            pui.vline(ts_vline.clone());

            if let Some(pointer) = pui.pointer_coordinate() {
                cursor_x = Some(pointer.x);
            }
        });

    ui.heading("Curvilinear Orientation");
    let _theta_plot = plot("curvilinear_plot")
        .center_y_axis(true)
        .label_formatter(angle_label_formatter)
        .show(ui, |pui| {
            pui.line(plot_data.curvilinear_orientation);
            pui.line(plot_data.curvilinear_orientation_ref);

            pui.vline(ts_vline.clone());

            if let Some(pointer) = pui.pointer_coordinate() {
                cursor_x = Some(pointer.x);
            }
        });

    ui.heading("Curvature");
    let _theta_plot = plot("kappa_plot")
        .center_y_axis(true)
        .label_formatter(unit_label_formatter("1/m"))
        .show(ui, |pui| {
            pui.line(plot_data.kappa);
            pui.line(plot_data.kappa_ref);

            pui.vline(ts_vline.clone());

            let kappa_max = f32::tan(vparams.delta_max) / vparams.wheelbase;

            pui.hline(
                egui_plot::HLine::new(kappa_max)
                    .style(egui_plot::LineStyle::Dashed { length: 10.0 }),
            );
            pui.hline(
                egui_plot::HLine::new(-kappa_max)
                    .style(egui_plot::LineStyle::Dashed { length: 10.0 }),
            );

            if let Some(pointer) = pui.pointer_coordinate() {
                cursor_x = Some(pointer.x);
            }
        });
    cursor_x
}

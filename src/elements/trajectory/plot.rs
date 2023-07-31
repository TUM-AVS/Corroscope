use bevy::prelude::*;

use egui::plot::PlotPoint;

pub(crate) struct TrajectoryPlotData {
    velocity: egui::plot::Line,
    velocity_ref: egui::plot::Line,
    acceleration: egui::plot::Line,
    acceleration_ref: egui::plot::Line,
    orientation: egui::plot::Line,
    orientation_ref: egui::plot::Line,
    curvilinear_orientation: egui::plot::Line,
    curvilinear_orientation_ref: egui::plot::Line,
    kappa: egui::plot::Line,
    kappa_ref: egui::plot::Line,
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
        }
    }
}

impl TrajectoryPlotData {
    pub(crate) fn from_data(plot_data: &CachedTrajectoryPlotData) -> Self {
        let velocity = egui::plot::Line::new(plot_data.velocity.clone()).name("velocity [m/s]");

        let velocity_ref =
            egui::plot::Line::new(plot_data.velocity_ref.clone()).name("reference velocity [m/s]");

        let acceleration = egui::plot::Line::new(plot_data.acceleration.clone())
            .name("acceleration [m/s\u{00B2}]");

        let acceleration_ref = egui::plot::Line::new(plot_data.acceleration_ref.clone())
            .name("reference acceleration [m/s\u{00B2}]");

        let orientation =
            egui::plot::Line::new(plot_data.orientation.clone()).name("global orientation [rad]");

        let orientation_ref = egui::plot::Line::new(plot_data.orientation_ref.clone())
            .style(egui::plot::LineStyle::Dotted { spacing: 6.0 })
            .name("reference global orientation [rad]");

        let curvilinear_orientation =
            egui::plot::Line::new(plot_data.curvilinear_orientation.clone())
                .name("curvilinear orientation [rad]");

        let curvilinear_orientation_ref =
            egui::plot::Line::new(plot_data.curvilinear_orientation_ref.clone())
                .style(egui::plot::LineStyle::Dotted { spacing: 6.0 })
                .name("reference curvilinear orientation [rad]");

        let kappa = egui::plot::Line::new(plot_data.kappa.clone()).name("curvature [1/m]");

        let kappa_ref = egui::plot::Line::new(plot_data.kappa_ref.clone())
            .style(egui::plot::LineStyle::Dotted { spacing: 6.0 })
            .name("reference curvature [1/m]");

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
        }
    }
}

pub(crate) fn plot_traj(
    plot_data: TrajectoryPlotData,
    ui: &mut egui::Ui,
    time_step: f32,
) -> Option<f64> {
    let mut cursor_x = None;

    let group = egui::Id::new("trajectory plot group");

    let plot_width = ui.available_width();

    let plot = |name: &'static str| {
        egui::plot::Plot::new(name)
            .legend(egui::plot::Legend::default().position(egui::plot::Corner::LeftBottom))
            .allow_drag(false)
            .allow_zoom(false)
            .view_aspect(2.0)
            .min_size(egui::Vec2::new(150.0, 75.0))
            .sharp_grid_lines(true)
            .include_x(0.0)
            .include_y(0.0)
            .width(plot_width)
            // .height(250.0)
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
        .y_grid_spacer(egui::plot::uniform_grid_spacer(|_grid_input| {
            [10.0, 2.0, 0.5]
        }))
        .label_formatter(unit_label_formatter("m/s"))
        .show(ui, |pui| {
            pui.line(plot_data.velocity);
            pui.line(plot_data.velocity_ref);

            pui.vline(ts_vline.clone());

            // TODO: Pass from vehicle parameters
            let v_max = 36;
            pui.hline(
                egui::plot::HLine::new(v_max).style(egui::plot::LineStyle::Dashed { length: 10.0 }),
            ); // .name("v_max"));

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
            pui.hline(
                egui::plot::HLine::new(a_max).style(egui::plot::LineStyle::Dashed { length: 10.0 }),
            ); //.name("maximum acceleration"));
            pui.hline(
                egui::plot::HLine::new(-a_max)
                    .style(egui::plot::LineStyle::Dashed { length: 10.0 }),
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
        .label_formatter(angle_label_formatter)
        .show(ui, |pui| {
            pui.line(plot_data.kappa);
            pui.line(plot_data.kappa_ref);

            pui.vline(ts_vline.clone());

            // TODO: Pass from vehicle parameters
            let wheelbase = 2.971;
            let delta_max = 0.610865;
            let kappa_max = f64::tan(delta_max) / wheelbase;

            pui.hline(
                egui::plot::HLine::new(kappa_max)
                    .style(egui::plot::LineStyle::Dashed { length: 10.0 }),
            );
            pui.hline(
                egui::plot::HLine::new(-kappa_max)
                    .style(egui::plot::LineStyle::Dashed { length: 10.0 }),
            );

            if let Some(pointer) = pui.pointer_coordinate() {
                cursor_x = Some(pointer.x);
            }
        });
    cursor_x
}

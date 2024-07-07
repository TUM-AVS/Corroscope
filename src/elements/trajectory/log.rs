use bevy::prelude::*;

use bevy_prototype_lyon::prelude::Stroke;

fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let s: &str = serde::Deserialize::deserialize(deserializer)?;

    s.to_lowercase().parse::<bool>().map_err(D::Error::custom)
}

fn deserialize_float_list<'de, D>(deserializer: D) -> Result<Vec<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let s: std::borrow::Cow<'static, str> = serde::Deserialize::deserialize(deserializer)?;

    s.split(',')
        .map(|s| s.parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(D::Error::custom)
}

#[derive(Debug, serde::Deserialize, Clone, Default, Reflect)]
pub(crate) struct Costs {
    occ_pm_cost: f64,
    occ_um_cost: f64,
    occ_ve_cost: f64,
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
pub(crate) struct KinematicData {
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) x_positions_m: Vec<f32>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) y_positions_m: Vec<f32>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) theta_orientations_rad: Vec<f32>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) kappa_rad: Vec<f32>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) curvilinear_orientations_rad: Vec<f32>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) velocities_mps: Vec<f32>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) accelerations_mps2: Vec<f32>,
}

impl KinematicData {
    pub(crate) fn positions(&self) -> impl Iterator<Item = Vec2> + '_ {
        std::iter::zip(self.x_positions_m.as_slice(), self.y_positions_m.as_slice())
            .map(|(&x, &y)| Vec2::new(x, y))
    }

    fn make_plot_data(data: &[f32], shift: Option<i32>) -> Vec<[f64; 2]> {
        let shift = shift.unwrap_or(0);

        let pdata = data
            .iter()
            .enumerate()
            .map(|(x, y)| [(shift + x as i32) as f64, *y as f64])
            .collect();

        pdata
    }

    pub(crate) fn velocity_plot_data(&self, shift: Option<i32>) -> Vec<[f64; 2]> {
        Self::make_plot_data(&self.velocities_mps, shift)
    }

    pub(crate) fn acceleration_plot_data(&self, shift: Option<i32>) -> Vec<[f64; 2]> {
        Self::make_plot_data(&self.accelerations_mps2, shift)
    }

    pub(crate) fn orientation_plot_data(&self, shift: Option<i32>) -> Vec<[f64; 2]> {
        Self::make_plot_data(&self.theta_orientations_rad, shift)
    }

    pub(crate) fn kappa_plot_data(&self, shift: Option<i32>) -> Vec<[f64; 2]> {
        Self::make_plot_data(&self.kappa_rad, shift)
    }

    pub(crate) fn curvilinear_orientation_plot_data(&self, shift: Option<i32>) -> Vec<[f64; 2]> {
        Self::make_plot_data(&self.curvilinear_orientations_rad, shift)
    }
}

#[derive(Debug, serde::Deserialize, Clone, Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct TrajectoryLog {
    pub(crate) time_step: i32,
    pub(crate) trajectory_number: i32,
    pub(crate) unique_id: i32,
    #[serde(deserialize_with = "deserialize_bool")]
    pub(crate) feasible: bool,
    pub(crate) horizon: f64,
    pub(crate) dt: f64,
    // pub(crate) actual_traj_length: f64,

    #[serde(flatten)]
    pub(crate) kinematic_data: KinematicData,

    pub(crate) s_position_m: f64,
    pub(crate) d_position_m: f64,

    pub(crate) ego_risk: Option<f64>,
    pub(crate) obst_risk: Option<f64>,
    pub(crate) costs_cumulative_weighted: f64,

    #[serde(flatten)]
    pub(crate) costs: std::collections::HashMap<String, f64>,

    pub(crate) inf_kin_yaw_rate: f64,
    pub(crate) inf_kin_acceleration: f64,
    pub(crate) inf_kin_max_curvature: f64,
    pub(crate) inf_kin_max_curvature_rate: f64,
}

impl TrajectoryLog {
    fn stroke(color: Color, line_width: f32) -> Stroke {
        use bevy_prototype_lyon::prelude::*;

        let mut stroke = Stroke::new(color, line_width);
        stroke.options.tolerance = 10.0;
        // stroke.options.line_join = LineJoin::Round;
        // stroke.options.start_cap = LineCap::Round;
        // stroke.options.end_cap = LineCap::Round;
        stroke
    }

    pub(crate) fn normal_stroke(&self, max_cost: f64) -> Stroke {
        Self::stroke(self.color(max_cost), 0.01)
    }

    pub(crate) fn selected_stroke(&self, max_cost: f64) -> Stroke {
        Self::stroke(self.selected_color(max_cost), 0.02)
    }

   pub(crate) fn color(&self, max_cost: f64) -> Color {
        if self.feasible {
            let time_step_gradient = false;
            let old_gradient = false;
            if time_step_gradient {
                Color::rgba_u8(
                    170_u8.saturating_sub((self.time_step as u8).saturating_mul(3)),
                    60,
                    90_u8,
                    40,
                )
            } else if old_gradient {
                let unit_cost = self.costs_cumulative_weighted / max_cost;
                let cost_val = 37.0 + (unit_cost.fract() * 360.0);
                let c = Color::hsla(cost_val as f32, 0.7, 0.7, 0.4);
                bevy::log::trace!("color={:?}", c);
                c
            } else {
                const GRAD: colorous::Gradient = colorous::VIRIDIS;

                let unit_cost = self.costs_cumulative_weighted.log2() / max_cost.log2();
                bevy::log::debug!("unit={} cost={} max={}", unit_cost, self.costs_cumulative_weighted, max_cost);

                let grad_color = GRAD.eval_continuous(unit_cost);
                let c = Color::rgb_u8(grad_color.r, grad_color.g, grad_color.b);
                bevy::log::trace!("color={:?}", c);
                c.with_a(0.7)
            }
        } else {
            Color::rgba_u8(30, 70, 190, 100)
        }
    }

    pub(crate) fn selected_color(&self, max_cost: f64) -> Color {
        let base_color = self.color(max_cost).as_hsla();
        base_color + Color::hsla(0.0, 0.1, 0.15, 0.2)
    }

    pub(crate) fn velocity_plot_data(&self) -> Vec<[f64; 2]> {
        self.kinematic_data.velocity_plot_data(Some(self.time_step))
    }

    pub(crate) fn acceleration_plot_data(&self) -> Vec<[f64; 2]> {
        self.kinematic_data
            .acceleration_plot_data(Some(self.time_step))
    }

    pub(crate) fn orientation_plot_data(&self) -> Vec<[f64; 2]> {
        self.kinematic_data
            .orientation_plot_data(Some(self.time_step))
    }

    pub(crate) fn kappa_plot_data(&self) -> Vec<[f64; 2]> {
        self.kinematic_data.kappa_plot_data(Some(self.time_step))
    }

    pub(crate) fn curvilinear_orientation_plot_data(&self) -> Vec<[f64; 2]> {
        self.kinematic_data
            .curvilinear_orientation_plot_data(Some(self.time_step))
    }

    pub(crate) fn sorted_nonzero_costs<'a>(
        &'a self,
        cost_threshold: Option<f64>,
    ) -> impl IntoIterator<Item = (&'a str, f64)> {
        let (mut valid, invalid): (Vec<_>, Vec<_>) = self
            .costs
            .iter()
            .map(|(k, v)| (k.as_str(), *v))
            .filter(|(_k, v)| {
                if v.is_finite() {
                    if let Some(th) = cost_threshold {
                        *v > th
                    } else {
                        true
                    }
                } else {
                    true
                }
            })
            .partition(|(_k, v)| v.is_finite());

        valid.sort_by(|(_k1, v1), (_k2, v2)| v1.partial_cmp(v2).unwrap().reverse());

        valid.extend(invalid.into_iter());

        valid
    }
}

#[allow(dead_code, non_snake_case)]
#[derive(Debug, serde::Deserialize, Clone, Component, Default, Reflect)]
#[reflect(Component)]
pub struct MainLog {
    pub(crate) trajectory_number: i32,
    pub(crate) calculation_time_s: f64,
    pub(crate) x_position_vehicle_m: f32,
    pub(crate) y_position_vehicle_m: f32,
    #[serde(deserialize_with = "deserialize_bool")]
    pub(crate) optimal_trajectory: bool,
    pub(crate) percentage_feasible_traj: Option<f64>,
    // pub(crate) infeasible_kinematics_sum: f64,
    pub(crate) infeasible_sum: f64,
    pub(crate) inf_kin_acceleration: f64,
    pub(crate) inf_kin_negative_s_velocity: f64,
    pub(crate) inf_kin_max_s_idx: f64,
    pub(crate) inf_kin_negative_v_velocity: f64,
    pub(crate) inf_kin_max_curvature: f64,
    pub(crate) inf_kin_yaw_rate: f64,
    pub(crate) inf_kin_max_curvature_rate: f64,
    pub(crate) inf_kin_vehicle_acc: f64,
    pub(crate) inf_cartesian_transform: f64,
    // pub(crate) infeasible_collision: f64,

    #[serde(flatten)]
    pub(crate) kinematic_data: KinematicData,

    pub(crate) s_position_m: f64,
    pub(crate) d_position_m: f64,

    pub(crate) ego_risk: Option<f64>,
    pub(crate) obst_risk: Option<f64>,
    pub(crate) costs_cumulative_weighted: f64,

    #[serde(flatten)]
    pub(crate) costs: Costs,
}

#[allow(unused)]
pub(crate) fn read_log(
    path: &std::path::Path,
) -> Result<Vec<TrajectoryLog>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;

    let map = unsafe { memmap2::MmapOptions::new().populate().map(&file)? };
    let cursor = std::io::Cursor::new(map);

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(cursor);

    let res = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;

    Ok(res)
}

pub(crate) fn read_main_log(
    path: &std::path::Path,
) -> Result<Vec<MainLog>, Box<dyn std::error::Error>> {
    let mut rdr = csv::ReaderBuilder::new().delimiter(b';').from_path(path)?;

    let res = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;

    Ok(res)
}

#[allow(unused)]
#[derive(Resource)]
pub struct MainTrajectory {
    path: Vec<Vec2>,
    kinematic_data: KinematicData,
}

#[derive(Component)]
pub struct HoveredTrajectory;

pub(crate) fn reassemble_main_trajectory(mtraj: &[MainLog]) -> KinematicData {
    let x_positions_m = mtraj
        .iter()
        .map(|traj| traj.x_position_vehicle_m)
        .collect::<Vec<f32>>();
    let y_positions_m = mtraj
        .iter()
        .map(|traj| traj.y_position_vehicle_m)
        .collect::<Vec<f32>>();

    let velocities_mps = mtraj
        .iter()
        .map(|traj| traj.kinematic_data.velocities_mps.first().copied())
        .collect::<Option<Vec<f32>>>()
        .unwrap();

    let accelerations_mps2 = mtraj
        .iter()
        .map(|traj| traj.kinematic_data.accelerations_mps2.first().copied())
        .collect::<Option<Vec<f32>>>()
        .unwrap();

    let theta_orientations_rad = mtraj
        .iter()
        .map(|traj| traj.kinematic_data.theta_orientations_rad.first().copied())
        .collect::<Option<Vec<f32>>>()
        .unwrap();

    let kappa_rad = mtraj
        .iter()
        .map(|traj| traj.kinematic_data.kappa_rad.first().copied())
        .collect::<Option<Vec<f32>>>()
        .unwrap();

    let curvilinear_orientations_rad = mtraj
        .iter()
        .map(|traj| {
            traj.kinematic_data
                .curvilinear_orientations_rad
                .first()
                .copied()
        })
        .collect::<Option<Vec<f32>>>()
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

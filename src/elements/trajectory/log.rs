use bevy::prelude::*;

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
pub(crate) struct Costs {
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
pub(crate) struct KinematicData {
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) x_positions_m: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) y_positions_m: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) theta_orientations_rad: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) kappa_rad: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) curvilinear_orientations_rad: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) velocities_mps: Vec<f64>,
    #[serde(deserialize_with = "deserialize_float_list")]
    pub(crate) accelerations_mps2: Vec<f64>,
}

impl KinematicData {
    pub(crate) fn positions(&self) -> impl Iterator<Item = Vec2> + '_ {
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
    pub(crate) actual_traj_length: f64,

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
    // inf_kin_max_curvature_rate: f64,
}

impl TrajectoryLog {
    pub(crate) fn color(&self) -> Color {
        if self.feasible {
            let time_step_gradient = false;
            if time_step_gradient {
                Color::rgba_u8(
                    170_u8.saturating_sub((self.time_step as u8).saturating_mul(3)),
                    60,
                    90_u8,
                    40,
                )
            } else {
                let unit_cost = self.costs_cumulative_weighted / 20.0;
                let cost_val = 37.0 + (unit_cost.fract() * 360.0);
                let c = Color::hsla(
                    cost_val as f32,
                    0.7,
                    0.8,
                    0.4,
                );
                bevy::log::debug!("color={:?}", c);
                c
            }
        } else {
            Color::rgba_u8(
                30, 70, 190,
                100,
            )
        }
    }

    pub(crate) fn selected_color(&self) -> Color {
        let base_color = self.color().as_hsla();
        base_color + Color::hsla(0.0, 0.1, 0.3, 0.2)
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

    pub(crate) fn sorted_nonzero_costs<'a>(&'a self) -> impl Iterator<Item = (&'a str, f64)> {
        let mut cost_values = self
            .costs
            .iter()
            .map(|(k, v)| (k.as_str(), *v))
            .filter(|(_k, v)| *v > 1e-3)
            .collect::<Vec<(_, f64)>>();

        cost_values.sort_by(|(_k1,v1), (_k2,v2)| v1.partial_cmp(v2).unwrap() );

        cost_values.into_iter().rev()
    }
}

#[allow(dead_code, non_snake_case)]
#[derive(Debug, serde::Deserialize, Clone, Component, Default, Reflect)]
#[reflect(Component)]
pub struct MainLog {
    pub(crate) trajectory_number: i32,
    pub(crate) calculation_time_s: f64,
    pub(crate) x_position_vehicle_m: f64,
    pub(crate) y_position_vehicle_m: f64,
    #[serde(deserialize_with = "deserialize_bool")]

    pub(crate) optimal_trajectory: bool,
    pub(crate) percentage_feasible_traj: Option<f64>,
    pub(crate) infeasible_kinematics_sum: f64,
    pub(crate) inf_kin_acceleration: f64,
    pub(crate) inf_kin_negative_s_velocity: f64,
    pub(crate) inf_kin_max_s_idx: f64,
    pub(crate) inf_kin_negative_v_velocity: f64,
    pub(crate) inf_kin_max_curvature: f64,
    pub(crate) inf_kin_yaw_rate: f64,
    pub(crate) inf_kin_max_curvature_rate: f64,
    pub(crate) inf_kin_vehicle_acc: f64,
    pub(crate) inf_cartesian_transform: f64,
    pub(crate) infeasible_collision: f64,

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

pub(crate) fn read_log(path: &std::path::Path) -> Result<Vec<TrajectoryLog>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;

    let map = unsafe { memmap2::MmapOptions::new().populate().map(&file)? };
    let cursor = std::io::Cursor::new(map);

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(cursor);

    let res = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;

    Ok(res)
}

pub(crate) fn read_main_log(path: &std::path::Path) -> Result<Vec<MainLog>, Box<dyn std::error::Error>> {
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

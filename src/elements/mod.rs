use bevy::prelude::*;

pub(crate) mod lanelet;
pub(crate) mod obstacle;
pub(crate) mod ref_path;
pub(crate) mod trajectory;

pub struct ElementsPlugin;

impl Plugin for ElementsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, obstacle::spawn_obstacles)
            .add_systems(Startup, lanelet::spawn_lanelets)
            .add_systems(Startup, trajectory::spawn_trajectories)
            .add_systems(Startup, ref_path::spawn_ref_path)
            .add_systems(Update, trajectory::trajectory_group_visibility)
            .add_systems(Update, trajectory::trajectory_visibility)
            .add_systems(Update, trajectory::trajectory_tooltip)
            .add_systems(Update, trajectory::trajectory_window)
            // .add_systems(Update, obstacle::plot_obs)
            .add_systems(Update, obstacle::obstacle_tooltip)
            .add_systems(Update, obstacle::trajectory_animation)
            .add_systems(Update, ref_path::ref_path_tooltip);

        app.register_type::<trajectory::TrajectoryLog>()
            .register_type::<trajectory::MainLog>()
            .register_type::<trajectory::TrajectoryGroup>();
    }
}

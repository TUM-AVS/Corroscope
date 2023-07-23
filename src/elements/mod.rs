use bevy::prelude::*;

pub mod lanelet;
pub mod obstacle;
pub mod ref_path;
pub mod trajectory;

pub struct ElementsPlugin;

impl Plugin for ElementsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, obstacle::spawn_obstacles)
            .add_systems(Startup, lanelet::spawn_lanelets)
            .add_systems(Startup, trajectory::spawn_trajectories)
            .add_systems(Startup, ref_path::spawn_ref_path)
            .add_systems(Update, trajectory::highlight_trajectories)
            .add_systems(Update, trajectory::trajectory_tooltip)
            .add_systems(Update, trajectory::trajectory_window)
            .add_systems(Update, trajectory::update_selected_color)
            .add_systems(Update, obstacle::plot_obs)
            .add_systems(Update, obstacle::obstacle_tooltip)
            .add_systems(Update, obstacle::trajectory_animation);
    }
}

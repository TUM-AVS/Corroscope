use bevy::prelude::*;

pub mod lanelet;
pub mod obstacle;
pub mod ref_path;
pub mod trajectory;

pub struct ElementsPlugin;

impl Plugin for ElementsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(obstacle::spawn_obstacles)
            .add_startup_system(lanelet::spawn_lanelets)
            .add_startup_system(trajectory::spawn_trajectories)
            .add_startup_system(ref_path::spawn_ref_path)
            .add_system(trajectory::highlight_trajectories)
            .add_system(trajectory::trajectory_tooltip)
            .add_system(trajectory::trajectory_window)
            .add_system(trajectory::update_selected_color)
            .add_system(obstacle::plot_obs)
            .add_system(obstacle::obstacle_tooltip)
            .add_system(obstacle::trajectory_animation);
    }
}

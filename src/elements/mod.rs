use bevy::prelude::*;

pub(crate) mod lanelet;
pub(crate) mod obstacle;
pub(crate) mod ref_path;
pub(crate) mod trajectory;

pub struct ElementsPlugin;

impl Plugin for ElementsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<trajectory::TrajectorySortKey>()
            .init_resource::<trajectory::SortDirection>()
            .add_systems(Startup,
                (
                    obstacle::spawn_obstacles,
                    lanelet::spawn_lanelets,
                    trajectory::spawn_trajectories,
                    ref_path::spawn_ref_path,
                )
            )
            .add_systems(Update,
                (
                        trajectory::trajectory_group_visibility,
                        trajectory::trajectory_visibility,
                        trajectory::trajectory_tooltip,
                        obstacle::obstacle_tooltip,
                        obstacle::trajectory_animation,
                        ref_path::ref_path_tooltip,
                        // obstacle::plot_obs,
                )
            )
            .add_systems(Update,
                (
                    (
                        trajectory::trajectory_list,
                    ),
                    (
                        trajectory::trajectory_window,
                        trajectory::sort_trajectory_list,
                    ),
                ).chain()
            )
            .add_event::<trajectory::SelectTrajectoryEvent>()
            .add_systems(PostUpdate, trajectory::update_selected_trajectory);

        app.register_type::<trajectory::TrajectoryLog>()
            .register_type::<trajectory::MainLog>()
            .register_type::<trajectory::TrajectoryGroup>();
    }
}

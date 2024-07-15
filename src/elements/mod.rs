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
                    obstacle::spawn_obstacles.after(crate::camera_setup),
                    lanelet::spawn_lanelets,
                    trajectory::spawn_trajectories,
                    ref_path::spawn_ref_path,
                )
            )
            .add_systems(Update,
                (
                        trajectory::update_stroke,
                        trajectory::trajectory_group_visibility,
                        trajectory::trajectory_visibility,
                        trajectory::trajectory_tooltip,
                        obstacle::obstacle_tooltip,
                        obstacle::trajectory_animation,
                        ref_path::ref_path_tooltip,
                        // obstacle::plot_obs,
                        show_generic_tooltips,
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
            .add_systems(
                PostUpdate,
                fix_render_asset_usages.after(bevy_prototype_lyon::plugin::BuildShapes)
            )
            .add_event::<trajectory::SelectTrajectoryEvent>()
            .add_systems(PostUpdate, trajectory::update_selected_trajectory);

        app.register_type::<trajectory::TrajectoryLog>()
            .register_type::<trajectory::MainLog>()
            .register_type::<trajectory::TrajectoryGroup>();
    }
}

fn fix_render_asset_usages(
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<
        (&mut bevy::sprite::Mesh2dHandle),
        (Changed<bevy::sprite::Mesh2dHandle>, Or<(With<bevy_prototype_lyon::draw::Stroke>, With<bevy_prototype_lyon::draw::Fill>)>),
    >,
) {
    for mesh_handle in query.iter() {
        use bevy::render::render_asset::RenderAssetUsages;

        let m = meshes.get_mut(&mesh_handle.0).unwrap();
        m.asset_usage = RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD;
    }
}

#[derive(Clone, Component, Reflect)]
#[component(storage = "SparseSet")]
pub(crate) struct HoverTooltip {
    text: String,
}

impl HoverTooltip {
    fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
        }
    }

    fn bundle(text: impl Into<String>) -> impl Bundle {
        use bevy_mod_picking::prelude::*;

        (
            On::<Pointer<Over>>::target_insert(HoverTooltip::new(text)),
            On::<Pointer<Out>>::target_remove::<HoverTooltip>(),
            PickableBundle::default(),
            // RaycastPickTarget::default(),
        )
    }

}

pub(crate) fn show_generic_tooltips(mut contexts: bevy_egui::EguiContexts, tooltip_q: Query<(Entity, &HoverTooltip)>) {
    let ctx = contexts.ctx_mut();

    let base_id = egui::Id::new("ref path tooltip");

    let layer_id = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("generic tooltips"));

    for (entity, tooltip) in tooltip_q.iter() {
        let id = base_id.with(entity);
        egui::containers::show_tooltip(ctx, layer_id, id, |ui| {
            ui.heading(tooltip.text.clone());
        });
    }
}
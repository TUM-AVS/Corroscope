use bevy::prelude::*;

use bevy_prototype_lyon::prelude::*;

use bevy_mod_picking::prelude::*;

#[derive(Debug, serde::Deserialize, Clone, Component, Reflect)]
#[serde(transparent)]
pub struct RefPath {
    points: Vec<[f64; 2]>,
}

#[derive(Clone, Copy, Component, Reflect)]
#[component(storage = "SparseSet")]
pub struct HoveredRefPath;

pub fn ref_path_tooltip(mut contexts: bevy_egui::EguiContexts, ref_path_q: Query<&HoveredRefPath>) {
    let ctx = contexts.ctx_mut();

    if !ref_path_q.is_empty() {
        egui::containers::show_tooltip(ctx, egui::Id::new("ref path tooltip"), |ui| {
            ui.heading("Reference Path");
        });
    }
}

fn read_ref_path(path: &std::path::Path) -> Result<RefPath, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;

    let data: RefPath = serde_json::from_reader(file)?;

    for window in data.points.as_slice().windows(2) {
        let [w1, w2] = window else { unreachable!() };

        let v1 = glam::f64::DVec2::from(*w1);
        let v2 = glam::f64::DVec2::from(*w2);

        let diff = v1.distance(v2);

        bevy::log::debug!("dist={:>7.3} v1={} v2={}", diff, v1, v2);
    }

    Ok(data)
}

pub fn spawn_ref_path(mut commands: Commands, args: Res<crate::args::Args>) {
    let rp = match read_ref_path(&args.reference_path) {
        Ok(rp) => rp,
        Err(e) => {
            bevy::log::error!("Failed to read reference path: {}", e);
            return;
        }
    };

    let points = rp
        .points
        .iter()
        .map(|pt| Vec2::new(pt[0] as f32, pt[1] as f32))
        .collect();

    let reference_path_shape: shapes::Polygon = bevy_prototype_lyon::shapes::Polygon {
        points,
        closed: false,
    };

    let reference_path_color = Color::rgba_u8(
        70, 15, 210, //.saturating_add(traj.unique_id),
        200, //100_u8.saturating_sub((traj.time_step as u8)), //.saturating_mul(4)),
    );

    commands.spawn((
        Name::new("reference path"),
        ShapeBundle {
            path: GeometryBuilder::build_as(&reference_path_shape),
            spatial: SpatialBundle {
                transform: Transform::from_xyz(0.0, 0.0, 1e-3),
                ..default()
            },
            ..default()
        },
        Stroke::new(reference_path_color, 0.1),
        PickableBundle::default(),
        // RaycastPickTarget::default(),
        On::<Pointer<Over>>::target_insert(HoveredRefPath),
        On::<Pointer<Out>>::target_remove::<HoveredRefPath>(),
    ));
}

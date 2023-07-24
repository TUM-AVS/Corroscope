use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

#[derive(Debug, serde::Deserialize, Clone, Component, Reflect)]
#[serde(transparent)]
pub struct RefPath {
    points: Vec<[f64; 2]>,
}

fn read_ref_path(path: &std::path::Path) -> Result<RefPath, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path).unwrap();

    let data = serde_json::from_reader(file).unwrap();

    Ok(data)
}

pub fn spawn_ref_path(mut commands: Commands, args: Res<crate::args::Args>) {
    let rp = read_ref_path(&args.reference_path).unwrap();

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
            transform: Transform::from_xyz(0.0, 0.0, 1e-3),
            ..default()
        },
        Stroke::new(reference_path_color, 0.1),
    ));
}

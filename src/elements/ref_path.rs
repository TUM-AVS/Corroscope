use bevy::prelude::*;

use bevy_prototype_lyon::prelude::*;

use bevy_mod_picking::prelude::*;

#[derive(Debug, miniserde::Deserialize, Clone, Component, Reflect)]
pub struct RefPath {
    x: Vec<f32>,
    y: Vec<f32>,
}

#[derive(Clone, Copy, Component, Reflect)]
// #[component(storage = "SparseSet")]
pub struct HoveredRefPath;

pub fn ref_path_tooltip(mut contexts: bevy_egui::EguiContexts, ref_path_q: Query<&HoveredRefPath>) {
    let ctx = contexts.ctx_mut();

    if !ref_path_q.is_empty() {
        egui::containers::show_tooltip(ctx, egui::Id::new("ref path tooltip"), |ui| {
            ui.heading("Reference Path");
        });
    }
}

fn read_ref_path(args: &crate::args::Args) -> Result<RefPath, Box<dyn std::error::Error>> {
    let db_path = std::path::Path::join(&args.logs, "trajectories.db");
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
    )?;

    let data: RefPath = {
        let mut stmt = conn.prepare(
            "SELECT value FROM meta WHERE key = 'reference_path'"
        )?;

        stmt.query_row([], |row| {
            let rusqlite::types::ValueRef::Text(st) = row.get_ref(0)? else { todo!() };
            let rstr = std::str::from_utf8(st)?;
            let data: RefPath = miniserde::json::from_str(rstr).map_err(|err| {
                return rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err));
            })?;

            Ok(data)
        })?
    };

    conn.close().unwrap();

    // let file = std::fs::File::open(path).unwrap();

    // let data: RefPath = serde_json::from_reader(file).unwrap();
    /*
    for window in data.points.as_slice().windows(2) {
        let [w1, w2] = window else { unreachable!() };

        let v1 = glam::f64::DVec2::from(*w1);
        let v2 = glam::f64::DVec2::from(*w2);

        let diff = v1.distance(v2);

        bevy::log::debug!("dist={:>7.3} v1={} v2={}", diff, v1, v2);
    }
    */

    Ok(data)
}

pub fn spawn_ref_path(mut commands: Commands, args: Res<crate::args::Args>) {
    let rp = match read_ref_path(&args) {
        Ok(rp) => rp,
        Err(e) => {
            bevy::log::error!("Failed to read reference path: {}", e);
            return;
        }
    };

    let points = std::iter::zip(rp.x, rp.y)
        .map(|(x, y)| Vec2::new(x, y))
        .collect();

    let reference_path_shape = crate::extra_shapes::Polyline {
        points,
    };

    let reference_path_color = Color::rgba_u8(
        70, 15, 210,
        200,
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

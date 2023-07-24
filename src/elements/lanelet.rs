use bevy::prelude::*;

use bevy_prototype_lyon::prelude::*;

use crate::commonroad_pb;

#[derive(Component)]
pub struct LeftBound;

#[derive(Component)]
pub struct RightBound;

#[derive(Component)]
pub struct LaneletBackground;

#[derive(Component)]
pub struct Lanelet;

fn make_dashed(path: &lyon_path::Path, dash_length: f32, dash_ratio: f32) -> lyon_path::Path {
    use lyon_algorithms::measure::{PathMeasurements, SampleType};

    let measurements = PathMeasurements::from_path(path, 1e-3);
    let mut sampler = measurements.create_sampler(path, SampleType::Distance);
    let rb_length = sampler.length();

    debug_assert!(dash_ratio < 1.0);

    let dash_count = (rb_length / dash_length).ceil() as i32;

    let mut builder = lyon_path::Path::builder();

    for i in 0..dash_count {
        let dash_start = i as f32 * dash_length;
        let dash_end = i as f32 * dash_length + dash_ratio * dash_length;

        sampler.split_range(dash_start..dash_end, &mut builder);
    }

    builder.build()
}

fn spawn_bound(bound: &commonroad_pb::Bound, id: u32) -> Option<impl Bundle> {
    let bound_pts = bound
        .points
        .iter()
        .map(|p| Vec2::new(p.x as f32, p.y as f32));

    let rb_shape = bevy_prototype_lyon::shapes::Polygon {
        points: bound_pts.collect(),
        closed: false,
    };
    let rb_path = GeometryBuilder::build_as(&rb_shape);

    let dashed_path = Path(make_dashed(&rb_path.0, 0.8, 0.5));
    let short_dashed_path = Path(make_dashed(&rb_path.0, 0.2, 0.5));

    let marking_color = Color::CRIMSON;
    let stroke_opts = {
        let mut opts = StrokeOptions::default();
        opts.start_cap = LineCap::Round;
        opts.end_cap = LineCap::Round;
        opts.line_join = LineJoin::Round;
        opts.line_width = 0.1;
        opts.tolerance = 1e-4;
        opts
    };
    let normal_stroke = Stroke {
        color: marking_color,
        options: stroke_opts, // st 0.1
    };
    let broad_stroke = Stroke {
        color: marking_color,
        options: {
            let mut opts = stroke_opts;
            opts.line_width = 0.2;
            opts
        },
    };
    let light_stroke = Stroke {
        color: Color::LIME_GREEN.with_a(0.5),
        options: {
            let mut opts = stroke_opts;
            opts.line_width = 0.07;
            opts
        },
    };

    use crate::commonroad_pb::line_marking_enum::LineMarking;

    let (path, stroke) = match bound.line_marking() {
        LineMarking::Solid => (rb_path, normal_stroke),
        LineMarking::BroadSolid => (rb_path, broad_stroke),
        LineMarking::Dashed => (dashed_path, normal_stroke),
        LineMarking::BroadDashed => (dashed_path, normal_stroke),
        LineMarking::Unknown => {
            bevy::log::info!("lanelet bound has unknown line marking");

            (short_dashed_path, light_stroke)
        }
        LineMarking::NoMarking => {
            bevy::log::info!("lanelet bound has no line marking");

            (short_dashed_path, light_stroke)
        }
    };

    Some((
        ShapeBundle {
            path,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1e-1 + (id as f32 * 1e-2))),
            ..default()
        },
        stroke,
    ))
}

pub fn spawn_lanelet(commands: &mut Commands, lanelet: &commonroad_pb::Lanelet) {
    if let Some(stop_line) = &lanelet.stop_line {
        let stop_line_shape: shapes::Polygon = bevy_prototype_lyon::shapes::Polygon {
            points: stop_line.points.iter().map(Into::into).collect(),
            closed: false,
        };
        commands.spawn((
            Name::new("stop line"),
            ShapeBundle {
                path: GeometryBuilder::build_as(&stop_line_shape),
                transform: Transform::from_xyz(0.0, 0.0, 1.0),
                ..default()
            },
            Fill::color(Color::BLACK),
        ));
    };

    let lbound_pts = lanelet.left_bound.points.iter().map(Into::<Vec2>::into);
    let rbound_pts = lanelet.right_bound.points.iter().map(Into::<Vec2>::into);

    let mut fpoints: Vec<Vec2> = vec![];
    fpoints.extend(lbound_pts.clone());
    fpoints.extend(rbound_pts.clone().rev());

    let ll_shape: shapes::Polygon = bevy_prototype_lyon::shapes::Polygon {
        points: fpoints,
        closed: false,
    };

    let main_entity = commands
        .spawn((Name::new("lanelet"), Lanelet, SpatialBundle::default()))
        .id();

    commands
        .spawn((
            Name::new("background"),
            LaneletBackground,
            ShapeBundle {
                path: GeometryBuilder::build_as(&ll_shape),
                transform: Transform::from_xyz(0.0, 0.0, 1.0),
                ..default()
            },
            Fill::color(Color::GRAY),
        ))
        .set_parent_in_place(main_entity);

    if let Some(bound) = spawn_bound(&lanelet.left_bound, lanelet.lanelet_id) {
        commands
            .spawn((Name::new("left bound"), LeftBound, bound))
            .set_parent(main_entity);
    }
    if let Some(bound) = spawn_bound(&lanelet.right_bound, lanelet.lanelet_id) {
        commands
            .spawn((Name::new("right bound"), RightBound, bound))
            .set_parent(main_entity);
    }
    /*
    commands.spawn((
        LeftBound,
        ShapeBundle {
            path: GeometryBuilder::build_as(&lb_shape),
            ..default()
        },
        Stroke::new(Color::CYAN, 0.1),
    ));

    commands.spawn((
        RightBound,
        ShapeBundle {
            path: Path(dashed_path), // GeometryBuilder::build_as(&rb_shape),
            ..default()
        },
        Stroke::new(Color::CYAN, 0.1),
    ));
    */
}

pub fn spawn_lanelets(mut commands: Commands, cr: Res<crate::CommonRoad>) {
    for lanelet in &cr.lanelets {
        spawn_lanelet(&mut commands, lanelet);
    }
}

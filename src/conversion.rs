use bevy::math::Vec2;

use crate::commonroad_pb;

use commonroad_pb::{integer_exact_or_interval, float_exact_or_interval};

impl From<commonroad_pb::Point> for Vec2 {
    fn from(value: commonroad_pb::Point) -> Self {
        Vec2::new(value.x as f32, value.y as f32)
    }
}

impl From<commonroad_pb::Point> for egui::Pos2 {
    fn from(value: commonroad_pb::Point) -> Self {
        egui::Pos2::new(value.x as f32, value.y as f32)
    }
}

impl TryFrom<commonroad_pb::state::Position> for egui::Pos2 {
    type Error = ();

    fn try_from(value: commonroad_pb::state::Position) -> Result<Self, Self::Error> {
        match value {
            commonroad_pb::state::Position::Point(p) => { Ok(p.into()) }
            _ => { Err(()) },
        }
    }
}

impl TryFrom<commonroad_pb::FloatExactOrInterval> for f64 {
    type Error = ();

    fn try_from(value: commonroad_pb::FloatExactOrInterval) -> Result<Self, Self::Error> {
        TryFrom::try_from(value.exact_or_interval.ok_or(())?)
    }
}

impl TryFrom<commonroad_pb::float_exact_or_interval::ExactOrInterval> for f64 {
    type Error = ();

    fn try_from(value: commonroad_pb::float_exact_or_interval::ExactOrInterval) -> Result<Self, Self::Error> {
        match value {
            float_exact_or_interval::ExactOrInterval::Exact(e) => Ok(e),
            _ => Err(()) ,
        }
    }
}


impl TryFrom<commonroad_pb::IntegerExactOrInterval> for i32 {
    type Error = ();

    fn try_from(value: commonroad_pb::IntegerExactOrInterval) -> Result<Self, Self::Error> {
        TryFrom::try_from(value.exact_or_interval.ok_or(())?)
    }
}

impl TryFrom<commonroad_pb::integer_exact_or_interval::ExactOrInterval> for i32 {
    type Error = ();

    fn try_from(value: commonroad_pb::integer_exact_or_interval::ExactOrInterval) -> Result<Self, Self::Error> {
        match value {
            integer_exact_or_interval::ExactOrInterval::Exact(e) => Ok(e),
            _ => Err(()) ,
        }
    }
}


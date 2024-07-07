use bevy::prelude::*;

use bevy_prototype_lyon::prelude::*;

use  bevy_prototype_lyon::shapes::RectangleOrigin;

use lyon_path::geom::LineSegment;
use lyon_path::math::{Point, Box2D};
use lyon_path::geom::euclid::Size2D;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoundedRectangle {
    pub extents: Vec2,
    pub origin: RectangleOrigin,
    pub radius: f32,
}

impl Default for RoundedRectangle {
    fn default() -> Self {
        Self {
            extents: Vec2::ONE,
            origin: RectangleOrigin::default(),
            radius: 0.0,
        }
    }
}

impl Geometry for RoundedRectangle {
    fn add_geometry(&self, b: &mut lyon_path::path::Builder) {

        let origin = match self.origin {
            RectangleOrigin::Center => Point::new(-self.extents.x / 2.0, -self.extents.y / 2.0),
            RectangleOrigin::BottomLeft => Point::new(0.0, 0.0),
            RectangleOrigin::BottomRight => Point::new(-self.extents.x, 0.0),
            RectangleOrigin::TopRight => Point::new(-self.extents.x, -self.extents.y),
            RectangleOrigin::TopLeft => Point::new(0.0, -self.extents.y),
            RectangleOrigin::CustomCenter(v) => {
                Point::new(v.x - self.extents.x / 2.0, v.y - self.extents.y / 2.0)
            }
        };

        b.add_rounded_rectangle(
            &Box2D::from_origin_and_size(origin, Size2D::new(self.extents.x, self.extents.y)),
            &lyon_path::builder::BorderRadii::new(self.radius),
            lyon_path::Winding::Positive,
        );
    }
}

#[derive(Debug, Default, Clone)]
pub struct Polyline {
    pub points: Vec<bevy::math::Vec2>,
}

impl Geometry for Polyline {
    fn add_geometry(&self, b: &mut lyon_path::path::Builder) {
        /*
        for window in self.points.as_slice().windows(2) {
            let first = window[0];
            let second = window[1];
            let lseg = LineSegment {
                from: Point::new(first.x, first.y),
                to: Point::new(second.x, second.y),
            };

            // b.add_line_segment(&lseg);
        }
         */
        b.reserve(self.points.len(), 0);

        let pt0 = self.points.first().unwrap();
        let pt0 = Point::new(pt0.x, pt0.y);
        b.begin(pt0);

        for pt in &self.points[1..] {
            let pt = Point::new(pt.x, pt.y);

            b.line_to(pt);
            // b.add_circle(pt, 0.01, lyon_path::Winding::Positive);
        }

        b.end(false);
    }
}

// #[derive(Debug, Default, Clone)]
pub struct PolylineRef<'a, T> where T: Copy + Into<bevy::math::Vec2> {
    slice: &'a [T],
    //pub points: Vec<bevy::math::Vec2>,
}

/*
impl<'a, T> Geometry for PolylineRef<'a, T> {
    fn add_geometry(&self, b: &mut lyon_path::path::Builder) {
        for window in self.slice.windows(2) {
            let first: Vec2 = window[0].into();
            let second: Vec2 = window[1].into();
            let lseg = LineSegment {
                from: Point::new(first.x, first.y),
                to: Point::new(second.x, second.y),
            };

            b.add_line_segment(&lseg);
        }
    }
}
 */
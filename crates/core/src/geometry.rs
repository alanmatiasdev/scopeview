//! Shapes in image pixel coordinates.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub a: Point,
    pub b: Point,
}

impl Segment {
    pub fn new(a: Point, b: Point) -> Self {
        Self { a, b }
    }

    /// Euclidean length in pixels.
    pub fn length(&self) -> f64 {
        (self.b.x - self.a.x).hypot(self.b.y - self.a.y)
    }

    pub fn midpoint(&self) -> Point {
        Point::new((self.a.x + self.b.x) / 2.0, (self.a.y + self.b.y) / 2.0)
    }

    /// Shortest distance from `p` to the segment (not the infinite line).
    pub fn distance_to(&self, p: Point) -> f64 {
        let (dx, dy) = (self.b.x - self.a.x, self.b.y - self.a.y);
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 {
            0.0
        } else {
            (((p.x - self.a.x) * dx + (p.y - self.a.y) * dy) / len2).clamp(0.0, 1.0)
        };
        (p.x - (self.a.x + t * dx)).hypot(p.y - (self.a.y + t * dy))
    }
}

/// Estimated width of one character, as a fraction of the text size.
pub const TEXT_CHAR_WIDTH: f64 = 0.55;
/// Estimated height of one line, as a fraction of the text size.
pub const TEXT_LINE_HEIGHT: f64 = 1.2;

/// A line of text placed on the image. `pos` is the top-left of the text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextNote {
    pub pos: Point,
    pub text: String,
}

impl TextNote {
    pub fn new(pos: Point, text: &str) -> Self {
        Self {
            pos,
            text: text.to_string(),
        }
    }

    /// Estimated size of the text block in image pixels, for hit testing. The renderers lay the
    /// text out with the real font; this only has to be close.
    pub fn estimated_size(&self, text_size: f64) -> (f64, f64) {
        (
            self.text.chars().count() as f64 * text_size * TEXT_CHAR_WIDTH,
            text_size * TEXT_LINE_HEIGHT,
        )
    }

    /// Distance from `p` to the estimated text box; zero inside it.
    pub fn distance_to(&self, p: Point, text_size: f64) -> f64 {
        let (w, h) = self.estimated_size(text_size);
        let dx = (self.pos.x - p.x).max(p.x - (self.pos.x + w)).max(0.0);
        let dy = (self.pos.y - p.y).max(p.y - (self.pos.y + h)).max(0.0);
        dx.hypot(dy)
    }
}

/// A freehand stroke: the points the pointer passed through, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub points: Vec<Point>,
}

impl Stroke {
    pub fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    /// Shortest distance from `p` to the drawn path (infinite when the stroke is empty).
    pub fn distance_to(&self, p: Point) -> f64 {
        match self.points.as_slice() {
            [] => f64::INFINITY,
            [only] => (only.x - p.x).hypot(only.y - p.y),
            points => points
                .windows(2)
                .map(|w| Segment::new(w[0], w[1]).distance_to(p))
                .fold(f64::INFINITY, f64::min),
        }
    }

    pub fn translated(&self, dx: f64, dy: f64) -> Stroke {
        Stroke::new(
            self.points
                .iter()
                .map(|q| Point::new(q.x + dx, q.y + dy))
                .collect(),
        )
    }
}

/// Axis-aligned rectangle given by two opposite corners, in any order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectShape {
    pub a: Point,
    pub b: Point,
}

impl RectShape {
    pub fn new(a: Point, b: Point) -> Self {
        Self { a, b }
    }

    pub fn width(&self) -> f64 {
        (self.b.x - self.a.x).abs()
    }

    pub fn height(&self) -> f64 {
        (self.b.y - self.a.y).abs()
    }

    /// Area in square pixels.
    pub fn area(&self) -> f64 {
        self.width() * self.height()
    }

    /// Top-left corner (smallest x, smallest y); labels are anchored here.
    pub fn top_left(&self) -> Point {
        Point::new(self.a.x.min(self.b.x), self.a.y.min(self.b.y))
    }
}

/// Circle given by its center and one point on the edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub center: Point,
    pub edge: Point,
}

impl Circle {
    pub fn new(center: Point, edge: Point) -> Self {
        Self { center, edge }
    }

    pub fn radius(&self) -> f64 {
        (self.edge.x - self.center.x).hypot(self.edge.y - self.center.y)
    }

    pub fn diameter(&self) -> f64 {
        self.radius() * 2.0
    }

    pub fn circumference(&self) -> f64 {
        std::f64::consts::TAU * self.radius()
    }

    /// Area in square pixels.
    pub fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius() * self.radius()
    }
}

/// Two circles sharing a center; each radius is set by its own edge point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Concentric {
    pub center: Point,
    pub edge1: Point,
    pub edge2: Point,
}

impl Concentric {
    pub fn new(center: Point, edge1: Point, edge2: Point) -> Self {
        Self {
            center,
            edge1,
            edge2,
        }
    }

    pub fn radius1(&self) -> f64 {
        Circle::new(self.center, self.edge1).radius()
    }

    pub fn radius2(&self) -> f64 {
        Circle::new(self.center, self.edge2).radius()
    }
}

/// A segment and a second segment parallel to it, centered on `through`.
/// The second segment has the same length and direction as the first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParallelLines {
    pub first: Segment,
    pub through: Point,
}

impl ParallelLines {
    pub fn new(first: Segment, through: Point) -> Self {
        Self { first, through }
    }

    /// Vector from the first line to the second (perpendicular to the first).
    /// With a zero-length first segment it points from the segment's point to `through`.
    pub fn offset(&self) -> (f64, f64) {
        let a = self.first.a;
        let (dx, dy) = (self.first.b.x - a.x, self.first.b.y - a.y);
        let (px, py) = (self.through.x - a.x, self.through.y - a.y);
        let len2 = dx * dx + dy * dy;
        if len2 == 0.0 {
            return (px, py);
        }
        let t = (px * dx + py * dy) / len2;
        (px - t * dx, py - t * dy)
    }

    /// Perpendicular distance between the two lines, in pixels.
    pub fn distance(&self) -> f64 {
        let (ox, oy) = self.offset();
        ox.hypot(oy)
    }

    /// Where the perpendicular from `through` meets the first line (extended if needed).
    pub fn foot(&self) -> Point {
        let (ox, oy) = self.offset();
        Point::new(self.through.x - ox, self.through.y - oy)
    }

    /// The second line: same length and direction as the first, centered on `through`.
    pub fn second(&self) -> Segment {
        let (dx, dy) = (
            (self.first.b.x - self.first.a.x) / 2.0,
            (self.first.b.y - self.first.a.y) / 2.0,
        );
        Segment::new(
            Point::new(self.through.x - dx, self.through.y - dy),
            Point::new(self.through.x + dx, self.through.y + dy),
        )
    }
}

/// Angle at `vertex` between the rays towards `a` and `b`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angle {
    pub vertex: Point,
    pub a: Point,
    pub b: Point,
}

impl Angle {
    pub fn new(vertex: Point, a: Point, b: Point) -> Self {
        Self { vertex, a, b }
    }

    /// The smaller angle between the rays, in degrees, from 0 to 180 (never the reflex angle).
    pub fn degrees(&self) -> f64 {
        let (ax, ay) = (self.a.x - self.vertex.x, self.a.y - self.vertex.y);
        let (bx, by) = (self.b.x - self.vertex.x, self.b.y - self.vertex.y);
        let cross = ax * by - ay * bx;
        let dot = ax * bx + ay * by;
        cross.abs().atan2(dot).to_degrees()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_length_is_euclidean() {
        let s = Segment::new(Point::new(0.0, 0.0), Point::new(3.0, 4.0));
        assert_eq!(s.length(), 5.0);
    }

    #[test]
    fn rect_corners_can_be_in_any_order() {
        let r = RectShape::new(Point::new(10.0, 8.0), Point::new(4.0, 2.0));
        assert_eq!(r.width(), 6.0);
        assert_eq!(r.height(), 6.0);
        assert_eq!(r.area(), 36.0);
        assert_eq!(r.top_left(), Point::new(4.0, 2.0));
    }

    #[test]
    fn degenerate_rect_has_zero_area() {
        let r = RectShape::new(Point::new(1.0, 1.0), Point::new(1.0, 9.0));
        assert_eq!(r.area(), 0.0);
    }

    #[test]
    fn text_box_distance_is_zero_inside() {
        let t = TextNote::new(Point::new(10.0, 10.0), "abcd");
        let (w, h) = t.estimated_size(20.0);
        assert!((w - 44.0).abs() < 1e-9 && (h - 24.0).abs() < 1e-9);
        assert_eq!(t.distance_to(Point::new(20.0, 20.0), 20.0), 0.0);
        assert_eq!(t.distance_to(Point::new(4.0, 10.0), 20.0), 6.0);
        assert_eq!(t.distance_to(Point::new(54.0 + 3.0, 34.0 + 4.0), 20.0), 5.0);
    }

    #[test]
    fn stroke_distance_follows_the_path() {
        let s = Stroke::new(vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ]);
        assert_eq!(s.distance_to(Point::new(5.0, 2.0)), 2.0);
        assert_eq!(s.distance_to(Point::new(12.0, 5.0)), 2.0);
        assert_eq!(
            Stroke::new(vec![]).distance_to(Point::new(0.0, 0.0)),
            f64::INFINITY
        );
        assert_eq!(
            Stroke::new(vec![Point::new(1.0, 1.0)]).distance_to(Point::new(4.0, 5.0)),
            5.0
        );
        let moved = s.translated(1.0, 2.0);
        assert_eq!(moved.points[0], Point::new(1.0, 2.0));
    }

    #[test]
    fn distance_to_segment_clamps_to_the_ends() {
        let s = Segment::new(Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        assert_eq!(s.distance_to(Point::new(5.0, 3.0)), 3.0);
        assert_eq!(s.distance_to(Point::new(13.0, 4.0)), 5.0);
        assert_eq!(s.midpoint(), Point::new(5.0, 0.0));
    }

    #[test]
    fn circle_values() {
        let c = Circle::new(Point::new(0.0, 0.0), Point::new(3.0, 4.0));
        assert_eq!(c.radius(), 5.0);
        assert_eq!(c.diameter(), 10.0);
        assert!((c.area() - 25.0 * std::f64::consts::PI).abs() < 1e-9);
        assert!((c.circumference() - 10.0 * std::f64::consts::PI).abs() < 1e-9);
    }

    #[test]
    fn concentric_radii() {
        let c = Concentric::new(
            Point::new(10.0, 10.0),
            Point::new(13.0, 14.0),
            Point::new(10.0, 30.0),
        );
        assert_eq!(c.radius1(), 5.0);
        assert_eq!(c.radius2(), 20.0);
    }

    #[test]
    fn parallel_distance_is_perpendicular() {
        // Horizontal line, pointer placed diagonally: only the vertical part counts.
        let p = ParallelLines::new(
            Segment::new(Point::new(0.0, 0.0), Point::new(10.0, 0.0)),
            Point::new(55.0, 7.0),
        );
        assert!((p.distance() - 7.0).abs() < 1e-9);
        // The second line is centered on the clicked point, with the first line's length.
        let second = p.second();
        assert_eq!(second.a, Point::new(50.0, 7.0));
        assert_eq!(second.b, Point::new(60.0, 7.0));
        // The connector meets the first line straight below the clicked point.
        assert_eq!(p.foot(), Point::new(55.0, 0.0));
    }

    #[test]
    fn parallel_works_for_slanted_lines() {
        let p = ParallelLines::new(
            Segment::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0)),
            Point::new(10.0, 0.0),
        );
        assert!((p.distance() - 50.0_f64.sqrt()).abs() < 1e-9);
        let s = p.second();
        let (dx, dy) = (s.b.x - s.a.x, s.b.y - s.a.y);
        assert!((dx - dy).abs() < 1e-9);
    }

    #[test]
    fn parallel_with_a_degenerate_first_line_uses_point_distance() {
        let p = ParallelLines::new(
            Segment::new(Point::new(1.0, 1.0), Point::new(1.0, 1.0)),
            Point::new(4.0, 5.0),
        );
        assert_eq!(p.distance(), 5.0);
    }

    #[test]
    fn angle_is_the_smaller_one() {
        let v = Point::new(0.0, 0.0);
        let right = Angle::new(v, Point::new(10.0, 0.0), Point::new(0.0, 10.0));
        assert!((right.degrees() - 90.0).abs() < 1e-9);
        // Same rays, swapped order: same value.
        let swapped = Angle::new(v, Point::new(0.0, 10.0), Point::new(10.0, 0.0));
        assert!((swapped.degrees() - 90.0).abs() < 1e-9);
        let obtuse = Angle::new(v, Point::new(10.0, 0.0), Point::new(-10.0, 10.0));
        assert!((obtuse.degrees() - 135.0).abs() < 1e-9);
        let straight = Angle::new(v, Point::new(10.0, 0.0), Point::new(-5.0, 0.0));
        assert!((straight.degrees() - 180.0).abs() < 1e-9);
        let same = Angle::new(v, Point::new(10.0, 0.0), Point::new(20.0, 0.0));
        assert!(same.degrees().abs() < 1e-9);
    }
}

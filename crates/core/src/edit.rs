//! Selecting and editing measurements: which part is under the pointer, and how a drag changes
//! the shape. Pure geometry in image pixels, so the rules are unit-tested.

use crate::measure::Measurement;
use crate::{Angle, Circle, Concentric, ParallelLines, Point, RectShape, Segment, TextNote};

/// What a pointer press grabbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grab {
    /// A handle, by index into [`handle_points`]. Dragging it reshapes the measurement.
    Handle(usize),
    /// The outline. Dragging it moves the whole measurement.
    Body,
}

fn rect_corners(r: &RectShape) -> [Point; 4] {
    [
        Point::new(r.a.x, r.a.y),
        Point::new(r.b.x, r.a.y),
        Point::new(r.b.x, r.b.y),
        Point::new(r.a.x, r.b.y),
    ]
}

/// The draggable points of a measurement, in handle-index order.
pub fn handle_points(m: &Measurement) -> Vec<Point> {
    match m {
        Measurement::Line(s) => vec![s.a, s.b],
        Measurement::Rect(r) => rect_corners(r).to_vec(),
        Measurement::Circle(c) => vec![c.center, c.edge],
        Measurement::Concentric(c) => vec![c.center, c.edge1, c.edge2],
        Measurement::Parallel(p) => vec![p.first.a, p.first.b, p.through],
        Measurement::Angle(a) => vec![a.vertex, a.a, a.b],
        // One handle marks the selection; dragging it moves the whole note or stroke.
        Measurement::Text(t) => vec![t.pos],
        Measurement::Pencil(s) => s.points.first().copied().into_iter().collect(),
    }
}

fn dist(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Distance from `p` to the drawn outline of the measurement.
fn body_distance(m: &Measurement, p: Point, text_size: f64) -> f64 {
    match m {
        Measurement::Line(s) => s.distance_to(p),
        Measurement::Rect(r) => {
            let c = rect_corners(r);
            (0..4)
                .map(|i| Segment::new(c[i], c[(i + 1) % 4]).distance_to(p))
                .fold(f64::INFINITY, f64::min)
        }
        Measurement::Circle(c) => (dist(c.center, p) - c.radius()).abs(),
        Measurement::Concentric(c) => (dist(c.center, p) - c.radius1())
            .abs()
            .min((dist(c.center, p) - c.radius2()).abs()),
        Measurement::Parallel(pl) => pl.first.distance_to(p).min(pl.second().distance_to(p)),
        Measurement::Angle(a) => Segment::new(a.vertex, a.a)
            .distance_to(p)
            .min(Segment::new(a.vertex, a.b).distance_to(p)),
        Measurement::Text(t) => t.distance_to(p, text_size),
        Measurement::Pencil(s) => s.distance_to(p),
    }
}

/// What is under `p` within `tolerance` pixels. A handle wins over the outline.
pub fn hit_test(m: &Measurement, p: Point, tolerance: f64, text_size: f64) -> Option<Grab> {
    let handle = handle_points(m)
        .iter()
        .enumerate()
        .map(|(i, h)| (i, dist(*h, p)))
        .filter(|(_, d)| *d <= tolerance)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((i, _)) = handle {
        return Some(Grab::Handle(i));
    }
    (body_distance(m, p, text_size) <= tolerance).then_some(Grab::Body)
}

fn shift(p: Point, dx: f64, dy: f64) -> Point {
    Point::new(p.x + dx, p.y + dy)
}

/// Move the whole measurement by (`dx`, `dy`).
pub fn translate(m: &Measurement, dx: f64, dy: f64) -> Measurement {
    match m.clone() {
        Measurement::Line(s) => {
            Measurement::Line(Segment::new(shift(s.a, dx, dy), shift(s.b, dx, dy)))
        }
        Measurement::Rect(r) => {
            Measurement::Rect(RectShape::new(shift(r.a, dx, dy), shift(r.b, dx, dy)))
        }
        Measurement::Circle(c) => {
            Measurement::Circle(Circle::new(shift(c.center, dx, dy), shift(c.edge, dx, dy)))
        }
        Measurement::Concentric(c) => Measurement::Concentric(Concentric::new(
            shift(c.center, dx, dy),
            shift(c.edge1, dx, dy),
            shift(c.edge2, dx, dy),
        )),
        Measurement::Parallel(p) => Measurement::Parallel(ParallelLines::new(
            Segment::new(shift(p.first.a, dx, dy), shift(p.first.b, dx, dy)),
            shift(p.through, dx, dy),
        )),
        Measurement::Angle(a) => Measurement::Angle(Angle::new(
            shift(a.vertex, dx, dy),
            shift(a.a, dx, dy),
            shift(a.b, dx, dy),
        )),
        Measurement::Text(t) => Measurement::Text(TextNote {
            pos: shift(t.pos, dx, dy),
            text: t.text,
        }),
        Measurement::Pencil(s) => Measurement::Pencil(s.translated(dx, dy)),
    }
}

/// Drag handle `index` to `to`. An unknown index leaves the measurement unchanged.
///
/// - Rectangle: the opposite corner stays fixed.
/// - Circle and concentric circles: the center handle moves the whole shape; an edge handle
///   changes that radius.
/// - Parallel lines: the first line's ends move it; the third handle sets the distance.
pub fn move_handle(m: &Measurement, index: usize, to: Point) -> Measurement {
    match (m.clone(), index) {
        (Measurement::Line(s), 0) => Measurement::Line(Segment::new(to, s.b)),
        (Measurement::Line(s), 1) => Measurement::Line(Segment::new(s.a, to)),
        (Measurement::Rect(r), i) if i < 4 => {
            let fixed = rect_corners(&r)[(i + 2) % 4];
            Measurement::Rect(RectShape::new(fixed, to))
        }
        (Measurement::Circle(c), 0) => translate(
            &Measurement::Circle(c),
            to.x - c.center.x,
            to.y - c.center.y,
        ),
        (Measurement::Circle(c), 1) => Measurement::Circle(Circle::new(c.center, to)),
        (Measurement::Concentric(c), 0) => translate(
            &Measurement::Concentric(c),
            to.x - c.center.x,
            to.y - c.center.y,
        ),
        (Measurement::Concentric(c), 1) => {
            Measurement::Concentric(Concentric::new(c.center, to, c.edge2))
        }
        (Measurement::Concentric(c), 2) => {
            Measurement::Concentric(Concentric::new(c.center, c.edge1, to))
        }
        (Measurement::Parallel(p), 0) => {
            Measurement::Parallel(ParallelLines::new(Segment::new(to, p.first.b), p.through))
        }
        (Measurement::Parallel(p), 1) => {
            Measurement::Parallel(ParallelLines::new(Segment::new(p.first.a, to), p.through))
        }
        (Measurement::Parallel(p), 2) => Measurement::Parallel(ParallelLines::new(p.first, to)),
        (Measurement::Angle(a), 0) => Measurement::Angle(Angle::new(to, a.a, a.b)),
        (Measurement::Angle(a), 1) => Measurement::Angle(Angle::new(a.vertex, to, a.b)),
        (Measurement::Angle(a), 2) => Measurement::Angle(Angle::new(a.vertex, a.a, to)),
        (Measurement::Text(t), 0) => translate(
            &Measurement::Text(t.clone()),
            to.x - t.pos.x,
            to.y - t.pos.y,
        ),
        (Measurement::Pencil(s), 0) => match s.points.first() {
            Some(first) => translate(
                &Measurement::Pencil(s.clone()),
                to.x - first.x,
                to.y - first.y,
            ),
            None => Measurement::Pencil(s),
        },
        _ => m.clone(),
    }
}

/// Apply a drag to the grabbed measurement. `original` is the shape when the drag started,
/// `from` the press position and `to` the current pointer, so repeated calls do not accumulate error.
pub fn apply_drag(original: &Measurement, grab: Grab, from: Point, to: Point) -> Measurement {
    match grab {
        Grab::Body => translate(original, to.x - from.x, to.y - from.y),
        Grab::Handle(i) => move_handle(original, i, to),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn line() -> Measurement {
        Measurement::Line(Segment::new(p(0.0, 0.0), p(100.0, 0.0)))
    }

    #[test]
    fn handle_beats_body_and_miss_is_none() {
        assert_eq!(
            hit_test(&line(), p(1.0, 1.0), 5.0, 18.0),
            Some(Grab::Handle(0))
        );
        assert_eq!(
            hit_test(&line(), p(99.0, 2.0), 5.0, 18.0),
            Some(Grab::Handle(1))
        );
        assert_eq!(hit_test(&line(), p(50.0, 3.0), 5.0, 18.0), Some(Grab::Body));
        assert_eq!(hit_test(&line(), p(50.0, 30.0), 5.0, 18.0), None);
    }

    #[test]
    fn rect_hits_corners_and_edges_but_not_the_inside() {
        let r = Measurement::Rect(RectShape::new(p(10.0, 10.0), p(110.0, 60.0)));
        assert_eq!(
            hit_test(&r, p(110.0, 60.0), 4.0, 18.0),
            Some(Grab::Handle(2))
        );
        assert_eq!(hit_test(&r, p(60.0, 11.0), 4.0, 18.0), Some(Grab::Body));
        assert_eq!(hit_test(&r, p(60.0, 35.0), 4.0, 18.0), None);
    }

    #[test]
    fn circle_hits_the_ring() {
        let c = Measurement::Circle(Circle::new(p(50.0, 50.0), p(80.0, 50.0)));
        assert_eq!(hit_test(&c, p(50.0, 21.0), 3.0, 18.0), Some(Grab::Body));
        assert_eq!(
            hit_test(&c, p(50.0, 50.0), 3.0, 18.0),
            Some(Grab::Handle(0))
        );
        assert_eq!(hit_test(&c, p(60.0, 50.0), 3.0, 18.0), None);
    }

    #[test]
    fn translate_moves_every_point() {
        let m = translate(&line(), 5.0, -2.0);
        assert_eq!(
            m,
            Measurement::Line(Segment::new(p(5.0, -2.0), p(105.0, -2.0)))
        );
    }

    #[test]
    fn dragging_a_rect_corner_keeps_the_opposite_one() {
        let r = Measurement::Rect(RectShape::new(p(10.0, 10.0), p(110.0, 60.0)));
        let moved = move_handle(&r, 2, p(200.0, 100.0));
        let Measurement::Rect(rect) = moved else {
            panic!("not a rect")
        };
        assert_eq!(rect.top_left(), p(10.0, 10.0));
        assert_eq!((rect.width(), rect.height()), (190.0, 90.0));
        // Dragging corner 0 keeps corner 2 fixed.
        let moved = move_handle(&r, 0, p(60.0, 30.0));
        let Measurement::Rect(rect) = moved else {
            panic!("not a rect")
        };
        assert_eq!((rect.width(), rect.height()), (50.0, 30.0));
    }

    #[test]
    fn circle_center_moves_it_and_edge_resizes() {
        let c = Measurement::Circle(Circle::new(p(50.0, 50.0), p(80.0, 50.0)));
        let Measurement::Circle(moved) = move_handle(&c, 0, p(100.0, 100.0)) else {
            panic!()
        };
        assert_eq!(moved.center, p(100.0, 100.0));
        assert_eq!(moved.radius(), 30.0);
        let Measurement::Circle(grown) = move_handle(&c, 1, p(50.0, 110.0)) else {
            panic!()
        };
        assert_eq!(grown.center, p(50.0, 50.0));
        assert_eq!(grown.radius(), 60.0);
    }

    #[test]
    fn parallel_third_handle_sets_the_distance() {
        let m = Measurement::Parallel(ParallelLines::new(
            Segment::new(p(0.0, 0.0), p(10.0, 0.0)),
            p(0.0, 5.0),
        ));
        let Measurement::Parallel(moved) = move_handle(&m, 2, p(3.0, 20.0)) else {
            panic!()
        };
        assert_eq!(moved.distance(), 20.0);
    }

    #[test]
    fn angle_handles_move_only_their_point() {
        let m = Measurement::Angle(Angle::new(p(0.0, 0.0), p(10.0, 0.0), p(0.0, 10.0)));
        let Measurement::Angle(moved) = move_handle(&m, 2, p(-10.0, 0.0)) else {
            panic!()
        };
        assert!((moved.degrees() - 180.0).abs() < 1e-9);
    }

    #[test]
    fn unknown_handle_changes_nothing() {
        assert_eq!(move_handle(&line(), 7, p(1.0, 1.0)), line());
    }

    #[test]
    fn drag_is_computed_from_the_original() {
        let moved = apply_drag(&line(), Grab::Body, p(10.0, 10.0), p(15.0, 20.0));
        let again = apply_drag(&line(), Grab::Body, p(10.0, 10.0), p(15.0, 20.0));
        assert_eq!(moved, again);
        assert_eq!(moved, translate(&line(), 5.0, 10.0));
    }

    #[test]
    fn text_is_grabbed_inside_its_box_and_by_its_handle() {
        let t = Measurement::Text(crate::TextNote::new(p(10.0, 10.0), "abcd"));
        // Box at size 20 spans x 10..54, y 10..34.
        assert_eq!(hit_test(&t, p(30.0, 20.0), 4.0, 20.0), Some(Grab::Body));
        assert_eq!(
            hit_test(&t, p(10.0, 10.0), 4.0, 20.0),
            Some(Grab::Handle(0))
        );
        assert_eq!(hit_test(&t, p(80.0, 20.0), 4.0, 20.0), None);
    }

    #[test]
    fn text_and_pencil_move_as_a_whole() {
        let t = Measurement::Text(crate::TextNote::new(p(10.0, 10.0), "hi"));
        let Measurement::Text(moved) = translate(&t, 5.0, -2.0) else {
            panic!("still text")
        };
        assert_eq!((moved.pos, moved.text.as_str()), (p(15.0, 8.0), "hi"));

        let s = Measurement::Pencil(crate::Stroke::new(vec![p(0.0, 0.0), p(4.0, 4.0)]));
        let Measurement::Pencil(moved) = move_handle(&s, 0, p(10.0, 10.0)) else {
            panic!("still a stroke")
        };
        assert_eq!(moved.points, vec![p(10.0, 10.0), p(14.0, 14.0)]);
    }

    #[test]
    fn pencil_is_grabbed_along_its_path() {
        let s = Measurement::Pencil(crate::Stroke::new(vec![
            p(0.0, 0.0),
            p(50.0, 0.0),
            p(50.0, 50.0),
        ]));
        assert_eq!(hit_test(&s, p(25.0, 2.0), 4.0, 18.0), Some(Grab::Body));
        assert_eq!(hit_test(&s, p(52.0, 30.0), 4.0, 18.0), Some(Grab::Body));
        assert_eq!(hit_test(&s, p(25.0, 30.0), 4.0, 18.0), None);
    }
}

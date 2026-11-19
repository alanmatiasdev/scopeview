//! Measurements the user draws on the image, in image pixel coordinates.

use crate::format::{
    angle_label, circle_labels, concentric_labels, line_label, parallel_label, rect_labels,
};
use crate::{
    Angle, Circle, Concentric, ParallelLines, Point, RectShape, Scale, Segment, Stroke, TextNote,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Measurement {
    Line(Segment),
    Rect(RectShape),
    Circle(Circle),
    Concentric(Concentric),
    Parallel(ParallelLines),
    Angle(Angle),
    /// Free text placed on the image; it measures nothing.
    Text(TextNote),
    /// Freehand drawing; it measures nothing.
    Pencil(Stroke),
}

impl Measurement {
    /// Label lines, in drawing order.
    pub fn labels(&self, scale: Option<&Scale>) -> Vec<String> {
        match self {
            Measurement::Line(s) => vec![line_label(s, scale)],
            Measurement::Rect(r) => rect_labels(r, scale).to_vec(),
            Measurement::Circle(c) => circle_labels(c, scale).to_vec(),
            Measurement::Concentric(c) => concentric_labels(c, scale).to_vec(),
            Measurement::Parallel(p) => vec![parallel_label(p, scale)],
            Measurement::Angle(a) => vec![angle_label(a)],
            Measurement::Text(t) => vec![t.text.clone()],
            Measurement::Pencil(_) => Vec::new(),
        }
    }

    /// Where the label is anchored: the first endpoint of a line, the top-left of a rectangle,
    /// the center of a circle, the second edge point of concentric circles, the point that places
    /// the second parallel line, the vertex of an angle,
    /// the top-left of a text, the first point of a stroke (which has no label).
    pub fn label_anchor(&self) -> Point {
        match self {
            Measurement::Line(s) => s.a,
            Measurement::Rect(r) => r.top_left(),
            Measurement::Circle(c) => c.center,
            Measurement::Concentric(c) => c.edge2,
            Measurement::Parallel(p) => p.through,
            Measurement::Angle(a) => a.vertex,
            Measurement::Text(t) => t.pos,
            Measurement::Pencil(s) => s.points.first().copied().unwrap_or(Point::new(0.0, 0.0)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MeasurementList {
    items: Vec<Measurement>,
}

impl MeasurementList {
    pub fn push(&mut self, m: Measurement) {
        self.items.push(m);
    }

    /// Remove and return the newest measurement.
    pub fn delete_last(&mut self) -> Option<Measurement> {
        self.items.pop()
    }

    pub fn get(&self, index: usize) -> Option<&Measurement> {
        self.items.get(index)
    }

    /// Replace the measurement at `index`; false when there is none.
    pub fn replace(&mut self, index: usize, m: Measurement) -> bool {
        match self.items.get_mut(index) {
            Some(slot) => {
                *slot = m;
                true
            }
            None => false,
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<Measurement> {
        (index < self.items.len()).then(|| self.items.remove(index))
    }

    /// The newest measurement under `p` (within `tolerance` image pixels) and what was grabbed.
    ///
    /// `text_size` is the label size in image pixels, used to estimate the box of text notes.
    pub fn hit(
        &self,
        p: Point,
        tolerance: f64,
        text_size: f64,
    ) -> Option<(usize, crate::edit::Grab)> {
        self.items
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, m)| crate::edit::hit_test(m, p, tolerance, text_size).map(|g| (i, g)))
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &Measurement> {
        self.items.iter()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point, Unit};

    fn line() -> Measurement {
        Measurement::Line(Segment::new(
            Point::new(10.0, 20.0),
            Point::new(110.0, 20.0),
        ))
    }

    fn rect() -> Measurement {
        Measurement::Rect(RectShape::new(
            Point::new(50.0, 40.0),
            Point::new(30.0, 10.0),
        ))
    }

    #[test]
    fn labels_follow_the_shape() {
        let s = Scale::from_calibration(100.0, 1.0, Unit::Mm).unwrap();
        assert_eq!(line().labels(Some(&s)), ["1.00mm"]);
        assert_eq!(rect().labels(None), ["20.00x30.00px", "S:600.00px^2"]);
    }

    #[test]
    fn new_shapes_have_labels_and_anchors() {
        let c = Measurement::Circle(Circle::new(Point::new(5.0, 5.0), Point::new(15.0, 5.0)));
        assert_eq!(c.labels(None)[0], "R:10.00px");
        assert_eq!(c.label_anchor(), Point::new(5.0, 5.0));
        let p = Measurement::Parallel(ParallelLines::new(
            Segment::new(Point::new(0.0, 0.0), Point::new(10.0, 0.0)),
            Point::new(0.0, 6.0),
        ));
        assert_eq!(p.labels(None), ["D:6.00px"]);
        assert_eq!(p.label_anchor(), Point::new(0.0, 6.0));
    }

    #[test]
    fn replace_remove_and_hit() {
        let mut list = MeasurementList::default();
        list.push(line());
        list.push(rect());
        assert_eq!(
            list.hit(Point::new(60.0, 20.5), 2.0, 18.0).map(|h| h.0),
            Some(0)
        );
        assert_eq!(list.hit(Point::new(300.0, 300.0), 2.0, 18.0), None);
        assert!(list.replace(0, rect()));
        assert!(!list.replace(9, rect()));
        assert_eq!(list.remove(0), Some(rect()));
        assert_eq!(list.remove(5), None);
        assert_eq!(list.len(), 1);
        assert_eq!(list.get(0), Some(&rect()));
    }

    #[test]
    fn anchors() {
        assert_eq!(line().label_anchor(), Point::new(10.0, 20.0));
        assert_eq!(rect().label_anchor(), Point::new(30.0, 10.0));
    }

    #[test]
    fn delete_last_and_clear() {
        let mut list = MeasurementList::default();
        list.push(line());
        list.push(rect());
        assert_eq!(list.delete_last(), Some(rect()));
        assert_eq!(list.len(), 1);
        list.clear();
        assert!(list.is_empty());
        assert_eq!(list.delete_last(), None);
    }
}

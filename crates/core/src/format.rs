//! Label text for measurements and the crosshair ruler.

use crate::geometry::{Angle, Circle, Concentric, ParallelLines};
use crate::{RectShape, Scale, Segment, Unit};

/// The two label lines of a rectangle: size and area.
///
/// With a scale the values are converted; without one they stay in pixels.
pub fn rect_labels(rect: &RectShape, scale: Option<&Scale>) -> [String; 2] {
    let (w, h, area, unit) = match scale {
        Some(s) => (
            s.length(rect.width()),
            s.length(rect.height()),
            s.area(rect.area()),
            s.unit(),
        ),
        None => (rect.width(), rect.height(), rect.area(), Unit::Px),
    };
    let suffix = unit.suffix();
    [
        format!("{w:.2}x{h:.2}{suffix}"),
        format!("S:{area:.2}{suffix}^2"),
    ]
}

/// The label of a straight line: its length with the unit, two decimals.
pub fn line_label(segment: &Segment, scale: Option<&Scale>) -> String {
    let (value, unit) = match scale {
        Some(s) => (s.length(segment.length()), s.unit()),
        None => (segment.length(), Unit::Px),
    };
    format!("{value:.2}{}", unit.suffix())
}

fn convert(value: f64, scale: Option<&Scale>) -> (f64, Unit) {
    match scale {
        Some(s) => (s.length(value), s.unit()),
        None => (value, Unit::Px),
    }
}

/// Circle labels: radius, circumference, area.
pub fn circle_labels(circle: &Circle, scale: Option<&Scale>) -> [String; 3] {
    let (r, unit) = convert(circle.radius(), scale);
    let (c, _) = convert(circle.circumference(), scale);
    let area = match scale {
        Some(s) => s.area(circle.area()),
        None => circle.area(),
    };
    let suffix = unit.suffix();
    [
        format!("R:{r:.2}{suffix}"),
        format!("C:{c:.2}{suffix}"),
        format!("S:{area:.2}{suffix}^2"),
    ]
}

/// Label of two concentric circles: both radii.
pub fn concentric_labels(c: &Concentric, scale: Option<&Scale>) -> [String; 2] {
    let (r1, unit) = convert(c.radius1(), scale);
    let (r2, _) = convert(c.radius2(), scale);
    let suffix = unit.suffix();
    [format!("R1:{r1:.2}{suffix}"), format!("R2:{r2:.2}{suffix}")]
}

/// Label of the distance between two parallel lines.
pub fn parallel_label(p: &ParallelLines, scale: Option<&Scale>) -> String {
    let (d, unit) = convert(p.distance(), scale);
    format!("D:{d:.2}{}", unit.suffix())
}

/// Label of an angle: degrees with two decimals. Calibration does not affect it.
pub fn angle_label(a: &Angle) -> String {
    format!("{:.2}°", a.degrees())
}

/// Label of a crosshair ruler step: one decimal for values from 0.1, two from 0.01, else three.
/// A small epsilon is added first so exact halves round up.
pub fn ruler_label(value: f64, unit: Unit) -> String {
    let suffix = unit.suffix();
    if value >= 0.1 {
        format!("{:.1}{suffix}", value + 0.01)
    } else if value >= 0.01 {
        format!("{:.2}{suffix}", value + 0.001)
    } else {
        format!("{:.3}{suffix}", value + 0.0001)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;

    fn rect(w: f64, h: f64) -> RectShape {
        RectShape::new(Point::new(5.0, 5.0), Point::new(5.0 + w, 5.0 + h))
    }

    #[test]
    fn calibrated_rect_labels() {
        let s = Scale::from_calibration(200.0, 2.0, Unit::Mm).unwrap();
        let labels = rect_labels(&rect(350.0, 200.0), Some(&s));
        assert_eq!(labels, ["3.50x2.00mm", "S:7.00mm^2"]);
    }

    #[test]
    fn calibrated_rect_labels_use_um() {
        let s = Scale::from_calibration(100.0, 50.0, Unit::Um).unwrap();
        let labels = rect_labels(&rect(100.0, 50.0), Some(&s));
        assert_eq!(labels, ["50.00x25.00µm", "S:1250.00µm^2"]);
    }

    #[test]
    fn uncalibrated_rect_labels_are_in_pixels() {
        let labels = rect_labels(&rect(30.0, 20.0), None);
        assert_eq!(labels, ["30.00x20.00px", "S:600.00px^2"]);
    }

    #[test]
    fn line_labels() {
        let seg = Segment::new(Point::new(0.0, 0.0), Point::new(300.0, 400.0));
        assert_eq!(line_label(&seg, None), "500.00px");
        let s = Scale::from_calibration(100.0, 1.0, Unit::Mm).unwrap();
        assert_eq!(line_label(&seg, Some(&s)), "5.00mm");
    }

    #[test]
    fn ruler_label_decimals_follow_magnitude() {
        assert_eq!(ruler_label(1.0, Unit::Mm), "1.0mm");
        assert_eq!(ruler_label(0.25, Unit::Mm), "0.3mm");
        assert_eq!(ruler_label(0.05, Unit::Mm), "0.05mm");
        assert_eq!(ruler_label(0.005, Unit::Mm), "0.005mm");
        assert_eq!(ruler_label(100.0, Unit::Um), "100.0µm");
    }

    #[test]
    fn ruler_label_boundaries() {
        assert_eq!(ruler_label(0.1, Unit::Mm), "0.1mm");
        assert_eq!(ruler_label(0.01, Unit::Mm), "0.01mm");
        assert_eq!(ruler_label(0.0099, Unit::Mm), "0.010mm");
    }

    #[test]
    fn circle_labels_in_units() {
        let s = Scale::from_calibration(100.0, 1.0, Unit::Mm).unwrap();
        let c = Circle::new(Point::new(0.0, 0.0), Point::new(100.0, 0.0));
        assert_eq!(
            circle_labels(&c, Some(&s)),
            ["R:1.00mm", "C:6.28mm", "S:3.14mm^2"]
        );
        assert_eq!(circle_labels(&c, None)[0], "R:100.00px");
    }

    #[test]
    fn concentric_parallel_and_angle_labels() {
        let c = Concentric::new(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(0.0, 25.0),
        );
        assert_eq!(concentric_labels(&c, None), ["R1:10.00px", "R2:25.00px"]);
        let p = ParallelLines::new(
            Segment::new(Point::new(0.0, 0.0), Point::new(10.0, 0.0)),
            Point::new(3.0, 8.0),
        );
        assert_eq!(parallel_label(&p, None), "D:8.00px");
        let a = Angle::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
        );
        assert_eq!(angle_label(&a), "90.00°");
    }
}

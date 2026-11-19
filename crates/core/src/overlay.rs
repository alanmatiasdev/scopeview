//! Turns measurements into drawing primitives. The app and the photo saver both render these,
//! so what is on screen is what ends up in the saved image.

use crate::measure::Measurement;
use crate::{Point, Scale};

pub type Rgba = [u8; 4];

/// Backdrop behind label text so it stays readable on any image.
pub const TEXT_BACKGROUND: Rgba = [0, 0, 0, 150];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub line_color: Rgba,
    /// Stroke width in image pixels.
    pub line_width: f32,
    pub text_color: Rgba,
    /// Text height in image pixels.
    pub text_size: f32,
    /// Color of the crosshair and its ticks.
    pub grid_color: Rgba,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            line_color: [255, 64, 64, 255],
            line_width: 2.0,
            text_color: [255, 255, 255, 255],
            text_size: 18.0,
            grid_color: [255, 220, 64, 255],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Line {
        a: Point,
        b: Point,
        color: Rgba,
        width: f32,
    },
    /// Unfilled rectangle; the stroke is drawn inside `min`..`max`.
    RectOutline {
        min: Point,
        max: Point,
        color: Rgba,
        width: f32,
    },
    /// Unfilled circle; the stroke is centered on the radius.
    Circle {
        center: Point,
        radius: f64,
        color: Rgba,
        width: f32,
    },
    /// Small filled square centered on `center`. Selection handles: drawn on screen only (`size` is
    /// in screen points there) and never rasterized into photos.
    Handle {
        center: Point,
        size: f32,
        color: Rgba,
    },
    /// Lines of text stacked downward from `pos` (the top-left of the block).
    Text {
        pos: Point,
        lines: Vec<String>,
        color: Rgba,
        size: f32,
        background: Rgba,
    },
}

fn line(a: Point, b: Point, style: &Style, width: f32) -> Primitive {
    Primitive::Line {
        a,
        b,
        color: style.line_color,
        width,
    }
}

fn circle(center: Point, radius: f64, style: &Style) -> Primitive {
    Primitive::Circle {
        center,
        radius,
        color: style.line_color,
        width: style.line_width,
    }
}

pub fn build_one(m: &Measurement, scale: Option<&Scale>, style: &Style) -> Vec<Primitive> {
    let w = style.line_width;
    let mut prims = match m {
        Measurement::Line(s) => vec![line(s.a, s.b, style, w)],
        Measurement::Rect(r) => {
            let min = r.top_left();
            vec![Primitive::RectOutline {
                min,
                max: Point::new(min.x + r.width(), min.y + r.height()),
                color: style.line_color,
                width: w,
            }]
        }
        // A radius line from the center to the edge point, plus the circle(s).
        Measurement::Circle(c) => vec![
            line(c.center, c.edge, style, w),
            circle(c.center, c.radius(), style),
        ],
        Measurement::Concentric(c) => vec![
            line(c.center, c.edge2, style, w),
            circle(c.center, c.radius1(), style),
            circle(c.center, c.radius2(), style),
        ],
        Measurement::Parallel(p) => {
            let second = p.second();
            vec![
                line(p.first.a, p.first.b, style, w),
                line(second.a, second.b, style, w),
                // The perpendicular shows the distance being measured.
                line(p.foot(), p.through, style, w),
            ]
        }
        Measurement::Angle(a) => vec![line(a.vertex, a.a, style, w), line(a.vertex, a.b, style, w)],
        Measurement::Text(_) => Vec::new(),
        Measurement::Pencil(s) => match s.points.as_slice() {
            [] => Vec::new(),
            // A single point is a dot: a zero-length line with round caps.
            [only] => vec![line(*only, *only, style, w)],
            points => points
                .windows(2)
                .map(|p| line(p[0], p[1], style, w))
                .collect(),
        },
    };
    let labels = m.labels(scale);
    if !labels.is_empty() {
        prims.push(Primitive::Text {
            pos: m.label_anchor(),
            lines: labels,
            color: style.text_color,
            size: style.text_size,
            background: TEXT_BACKGROUND,
        });
    }
    prims
}

/// Side of a selection handle, in screen points.
pub const HANDLE_SIZE: f32 = 10.0;

/// Handle squares for a selected measurement. Shown on screen only, never burned into photos.
pub fn handles(m: &Measurement, style: &Style) -> Vec<Primitive> {
    crate::edit::handle_points(m)
        .into_iter()
        .map(|center| Primitive::Handle {
            center,
            size: HANDLE_SIZE,
            color: style.text_color,
        })
        .collect()
}

pub fn build<'a>(
    measurements: impl IntoIterator<Item = &'a Measurement>,
    scale: Option<&Scale>,
    style: &Style,
) -> Vec<Primitive> {
    measurements
        .into_iter()
        .flat_map(|m| build_one(m, scale, style))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RectShape, Segment};

    #[test]
    fn rect_gives_outline_and_two_line_label_at_top_left() {
        let m = Measurement::Rect(RectShape::new(
            Point::new(40.0, 30.0),
            Point::new(10.0, 5.0),
        ));
        let prims = build([&m], None, &Style::default());
        assert_eq!(prims.len(), 2);
        assert!(matches!(
            &prims[0],
            Primitive::RectOutline { min, max, .. }
                if *min == Point::new(10.0, 5.0) && *max == Point::new(40.0, 30.0)
        ));
        assert!(matches!(
            &prims[1],
            Primitive::Text { pos, lines, .. } if *pos == Point::new(10.0, 5.0) && lines.len() == 2
        ));
    }

    #[test]
    fn line_label_sits_at_first_endpoint() {
        let m = Measurement::Line(Segment::new(Point::new(7.0, 9.0), Point::new(70.0, 9.0)));
        let prims = build_one(&m, None, &Style::default());
        assert!(matches!(
            &prims[1],
            Primitive::Text { pos, lines, .. } if *pos == Point::new(7.0, 9.0) && lines == &["63.00px"]
        ));
    }

    #[test]
    fn circle_shapes_give_circles_and_one_label() {
        let m = Measurement::Concentric(crate::Concentric::new(
            Point::new(50.0, 50.0),
            Point::new(60.0, 50.0),
            Point::new(80.0, 50.0),
        ));
        let prims = build_one(&m, None, &Style::default());
        let circles = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Circle { .. }))
            .count();
        assert_eq!(circles, 2);
        assert!(matches!(
            prims.last(),
            Some(Primitive::Text { lines, .. }) if lines == &["R1:10.00px", "R2:30.00px"]
        ));
    }

    #[test]
    fn text_note_is_one_label_in_the_text_style() {
        let m = Measurement::Text(crate::TextNote::new(Point::new(5.0, 6.0), "hello"));
        let prims = build_one(&m, None, &Style::default());
        assert_eq!(prims.len(), 1);
        assert!(matches!(
            &prims[0],
            Primitive::Text { pos, lines, size, .. }
                if *pos == Point::new(5.0, 6.0) && lines == &["hello"] && *size == 18.0
        ));
    }

    #[test]
    fn pencil_is_a_run_of_lines_without_a_label() {
        let m = Measurement::Pencil(crate::Stroke::new(vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 0.0),
        ]));
        let prims = build_one(&m, None, &Style::default());
        assert_eq!(prims.len(), 2);
        assert!(prims.iter().all(|p| matches!(p, Primitive::Line { .. })));
        let empty = Measurement::Pencil(crate::Stroke::new(vec![]));
        assert!(build_one(&empty, None, &Style::default()).is_empty());
    }

    #[test]
    fn handles_follow_the_shape() {
        let m = Measurement::Angle(crate::Angle::new(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(0.0, 10.0),
        ));
        assert_eq!(handles(&m, &Style::default()).len(), 3);
    }

    #[test]
    fn style_is_applied() {
        let style = Style {
            line_color: [1, 2, 3, 255],
            line_width: 5.0,
            ..Style::default()
        };
        let m = Measurement::Line(Segment::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0)));
        assert!(matches!(
            build_one(&m, None, &style)[0],
            Primitive::Line { color: [1, 2, 3, 255], width, .. } if width == 5.0
        ));
    }
}

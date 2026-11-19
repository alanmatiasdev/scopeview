//! The crosshair with ruler ticks, as overlay primitives centered on the image.

use crate::format::ruler_label;
use crate::overlay::{Primitive, Style, TEXT_BACKGROUND};
use crate::{Point, Scale};

/// Length on screen the ruler's major step aims for, in image pixels.
pub const TARGET_MAJOR_PX: f64 = 100.0;

/// Pick the major step of a calibrated ruler: among 1, 2 and 5 times a power of ten from 0.0001
/// to 10 (units), the value whose length is closest to [`TARGET_MAJOR_PX`]. The first of equally
/// close candidates (the smaller value) wins. Returns the value and its length rounded to whole
/// pixels, or `None` when the scale is unusable or the step would be under one pixel.
pub fn nice_step(pixels_per_unit: f64) -> Option<(f64, u32)> {
    if !(pixels_per_unit.is_finite() && pixels_per_unit > 0.0) {
        return None;
    }
    let mut best: Option<(f64, f64)> = None;
    for exponent in -4..=1 {
        for mantissa in [1.0, 2.0, 5.0] {
            let value = mantissa * 10f64.powi(exponent);
            let px = value * pixels_per_unit;
            let better = best
                .is_none_or(|(_, b)| (TARGET_MAJOR_PX - px).abs() < (TARGET_MAJOR_PX - b).abs());
            if better {
                best = Some((value, px));
            }
        }
    }
    let (value, px) = best?;
    let rounded = (px + 0.5) as u32;
    (rounded > 0).then_some((value, rounded))
}

fn tick(prims: &mut Vec<Primitive>, at: Point, len: f64, horizontal_axis: bool, style: &Style) {
    let half = len / 2.0;
    let (a, b) = if horizontal_axis {
        (Point::new(at.x, at.y - half), Point::new(at.x, at.y + half))
    } else {
        (Point::new(at.x - half, at.y), Point::new(at.x + half, at.y))
    };
    prims.push(Primitive::Line {
        a,
        b,
        color: style.grid_color,
        width: 1.0,
    });
}

/// Crosshair for an image of `size` pixels.
///
/// Uncalibrated: a tick every 10 px on both axes, long (9 px) on every 5th and short (4 px)
/// otherwise. Calibrated: major ticks (20 px) at a nice step with 9 minor ticks between them
/// (8 px, 14 px at the middle one), and one label at the first major tick right of the center.
/// Ticks are centered on the axis.
pub fn build(size: (u32, u32), scale: Option<&Scale>, style: &Style) -> Vec<Primitive> {
    let (w, h) = (f64::from(size.0), f64::from(size.1));
    let (cx, cy) = (w / 2.0, h / 2.0);
    let mut prims = vec![
        Primitive::Line {
            a: Point::new(0.0, cy),
            b: Point::new(w, cy),
            color: style.grid_color,
            width: 1.0,
        },
        Primitive::Line {
            a: Point::new(cx, 0.0),
            b: Point::new(cx, h),
            color: style.grid_color,
            width: 1.0,
        },
    ];

    let step = scale.and_then(|s| nice_step(s.pixels_per_unit()));
    match (scale, step) {
        (Some(scale), Some((value, major))) => {
            let major_px = f64::from(major);
            // Minor ticks first so majors draw on top of them.
            for (axis_len, horizontal) in [(w / 2.0, true), (h / 2.0, false)] {
                let minor_px = major_px / 10.0;
                let count = (axis_len / minor_px).floor() as i64;
                for i in 1..=count {
                    let len = if i % 10 == 0 {
                        20.0
                    } else if i % 5 == 0 {
                        14.0
                    } else {
                        8.0
                    };
                    for sign in [-1.0, 1.0] {
                        let d = sign * i as f64 * minor_px;
                        let at = if horizontal {
                            Point::new(cx + d, cy)
                        } else {
                            Point::new(cx, cy + d)
                        };
                        tick(&mut prims, at, len, horizontal, style);
                    }
                }
            }
            prims.push(Primitive::Text {
                // Anchored by its left edge; the app and photo renderer measure text themselves,
                // so we place the block's left edge at the first major tick.
                pos: Point::new(cx + major_px, cy + 2.0),
                lines: vec![ruler_label(value, scale.unit())],
                color: style.text_color,
                size: style.text_size,
                background: [
                    TEXT_BACKGROUND[0],
                    TEXT_BACKGROUND[1],
                    TEXT_BACKGROUND[2],
                    0,
                ],
            });
        }
        // Calibrated but with a step under one pixel: only the axes.
        (Some(_), None) => {}
        (None, _) => {
            for (axis_len, horizontal) in [(w / 2.0, true), (h / 2.0, false)] {
                let count = (axis_len / 10.0).floor() as i64;
                for i in 1..=count {
                    let len = if i % 5 == 0 { 9.0 } else { 4.0 };
                    for sign in [-1.0, 1.0] {
                        let d = sign * i as f64 * 10.0;
                        let at = if horizontal {
                            Point::new(cx + d, cy)
                        } else {
                            Point::new(cx, cy + d)
                        };
                        tick(&mut prims, at, len, horizontal, style);
                    }
                }
            }
        }
    }
    prims
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Unit;

    fn lines(prims: &[Primitive]) -> Vec<(Point, Point)> {
        prims
            .iter()
            .filter_map(|p| match p {
                Primitive::Line { a, b, .. } => Some((*a, *b)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn nice_step_is_the_candidate_closest_to_100_px() {
        // 100 px per mm: 1 mm is exactly 100 px.
        assert_eq!(nice_step(100.0), Some((1.0, 100)));
        // 30 px per mm: 2 mm = 60 px (off by 40) beats 5 mm = 150 px (off by 50).
        assert_eq!(nice_step(30.0), Some((2.0, 60)));
        // 500 px per mm: 0.2 mm = 100 px.
        let (v, px) = nice_step(500.0).unwrap();
        assert!((v - 0.2).abs() < 1e-12 && px == 100);
        // 2 px per mm: 50 mm = 100 px.
        assert_eq!(nice_step(2.0), Some((50.0, 100)));
    }

    #[test]
    fn nice_step_rejects_bad_scales() {
        assert_eq!(nice_step(0.0), None);
        assert_eq!(nice_step(f64::NAN), None);
        assert_eq!(nice_step(-1.0), None);
        // Even the largest candidate (50 units) would be under one pixel.
        assert_eq!(nice_step(0.001), None);
    }

    #[test]
    fn nice_step_stays_near_the_target_across_scales() {
        for ppu in [2.0, 7.7, 123.0, 9999.0, 1e6] {
            let (v, px) = nice_step(ppu).unwrap();
            assert!(v > 0.0 && (50..=160).contains(&px), "ppu {ppu}: {px} px");
        }
    }

    #[test]
    fn uncalibrated_axes_and_ticks() {
        let prims = build((200, 100), None, &Style::default());
        let l = lines(&prims);
        // Two axes, then ticks: horizontal axis half length 100 -> 10 per side, vertical 50 -> 5.
        assert_eq!(l.len(), 2 + 2 * 10 + 2 * 5);
        assert_eq!(l[0], (Point::new(0.0, 50.0), Point::new(200.0, 50.0)));
        assert_eq!(l[1], (Point::new(100.0, 0.0), Point::new(100.0, 100.0)));
        // First tick right of center is short (4 px), the fifth is long (9 px).
        assert_eq!(l[3], (Point::new(110.0, 48.0), Point::new(110.0, 52.0)));
        let fifth = l.iter().find(|(a, _)| a.x == 150.0 && a.y < 50.0).unwrap();
        assert_eq!(fifth.1.y - fifth.0.y, 9.0);
    }

    #[test]
    fn calibrated_ruler_has_majors_minors_and_one_label() {
        // 100 px per mm on a 1000 x 400 image: step 1 mm = 100 px, minors every 10 px.
        let s = Scale::from_calibration(100.0, 1.0, Unit::Mm).unwrap();
        let prims = build((1000, 400), Some(&s), &Style::default());
        let l = lines(&prims);
        let major_len = |l: &(Point, Point)| (l.1.y - l.0.y).abs();
        let majors = l[2..]
            .iter()
            .filter(|t| t.0.x == t.1.x && (major_len(t) - 20.0).abs() < 1e-9)
            .count();
        // Horizontal axis: 500 px / 100 = 5 majors per side = 10. (Vertical axis ticks are horizontal.)
        assert_eq!(majors, 10);
        let texts: Vec<_> = prims
            .iter()
            .filter_map(|p| match p {
                Primitive::Text { pos, lines, .. } => Some((*pos, lines.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].0, Point::new(600.0, 202.0));
        assert_eq!(texts[0].1, ["1.0mm"]);
    }

    #[test]
    fn minor_tick_lengths_follow_the_pattern() {
        let s = Scale::from_calibration(100.0, 1.0, Unit::Mm).unwrap();
        let prims = build((1000, 400), Some(&s), &Style::default());
        let l = lines(&prims);
        let at = |x: f64| {
            l.iter()
                .find(|t| t.0.x == x && t.1.x == x && t.0.y < 200.0 && t.0.y > 150.0)
                .map(|t| t.1.y - t.0.y)
        };
        assert_eq!(at(510.0), Some(8.0)); // minor
        assert_eq!(at(550.0), Some(14.0)); // middle one
        assert_eq!(at(600.0), Some(20.0)); // major
    }
}

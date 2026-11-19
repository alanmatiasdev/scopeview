//! Paints overlay primitives with egui, using the same primitives the photo saver rasterizes.

use eframe::egui::{self, Color32, FontId, Rect, Stroke, StrokeKind, Vec2};
use scopeview_core::overlay::{Primitive, Rgba};

use crate::view::ViewTransform;

fn color(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

pub fn paint(painter: &egui::Painter, view: &ViewTransform, primitives: &[Primitive]) {
    for primitive in primitives {
        match primitive {
            Primitive::Line {
                a,
                b,
                color: c,
                width,
            } => {
                painter.line_segment(
                    [view.to_screen(*a), view.to_screen(*b)],
                    Stroke::new(width * view.scale, color(*c)),
                );
            }
            Primitive::RectOutline {
                min,
                max,
                color: c,
                width,
            } => {
                let rect = Rect::from_min_max(view.to_screen(*min), view.to_screen(*max));
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(width * view.scale, color(*c)),
                    StrokeKind::Inside,
                );
            }
            Primitive::Circle {
                center,
                radius,
                color: c,
                width,
            } => {
                painter.circle_stroke(
                    view.to_screen(*center),
                    *radius as f32 * view.scale,
                    Stroke::new(width * view.scale, color(*c)),
                );
            }
            Primitive::Handle {
                center,
                size,
                color: c,
            } => {
                // Handles keep one size on screen whatever the zoom.
                let rect = Rect::from_center_size(view.to_screen(*center), Vec2::splat(*size));
                painter.rect_filled(rect, 0.0, color(*c));
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0, Color32::BLACK),
                    StrokeKind::Outside,
                );
            }
            Primitive::Text {
                pos,
                lines,
                color: c,
                size,
                background,
            } => {
                let mut at = view.to_screen(*pos);
                let pad = Vec2::new(3.0, 1.5) * view.scale;
                for line in lines {
                    let galley = painter.layout_no_wrap(
                        line.clone(),
                        FontId::proportional(size * view.scale),
                        color(*c),
                    );
                    let bg = Rect::from_min_size(at, galley.size()).expand2(pad);
                    painter.rect_filled(bg, 0.0, color(*background));
                    let height = galley.size().y;
                    painter.galley(at, galley, color(*c));
                    at.y += height;
                }
            }
        }
    }
}

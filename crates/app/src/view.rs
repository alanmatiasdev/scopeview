//! Mapping between image pixels and screen points, kept pure so it is testable.

use eframe::egui::{Pos2, Rect, Vec2};
use scopeview_core::Point;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewTransform {
    /// Where the image is drawn on screen.
    pub image_rect: Rect,
    /// Screen points per image pixel.
    pub scale: f32,
    image_size: (u32, u32),
}

impl ViewTransform {
    /// Fit an image of `image` pixels inside `available`, keeping its aspect ratio, centered.
    pub fn fit(available: Rect, image: (u32, u32)) -> Option<Self> {
        if image.0 == 0 || image.1 == 0 || available.width() <= 0.0 || available.height() <= 0.0 {
            return None;
        }
        let (w, h) = (image.0 as f32, image.1 as f32);
        let scale = (available.width() / w).min(available.height() / h);
        Some(Self {
            image_rect: Rect::from_center_size(available.center(), Vec2::new(w * scale, h * scale)),
            scale,
            image_size: image,
        })
    }

    pub fn to_screen(self, p: Point) -> Pos2 {
        self.image_rect.min + Vec2::new(p.x as f32, p.y as f32) * self.scale
    }

    /// Image pixel under a screen position (may lie outside the image).
    pub fn to_image(self, pos: Pos2) -> Point {
        let v = (pos - self.image_rect.min) / self.scale;
        Point::new(f64::from(v.x), f64::from(v.y))
    }

    /// Like `to_image`, limited to the image area.
    pub fn to_image_clamped(self, pos: Pos2) -> Point {
        let p = self.to_image(pos);
        Point::new(
            p.x.clamp(0.0, f64::from(self.image_size.0)),
            p.y.clamp(0.0, f64::from(self.image_size.1)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: f32, h: f32) -> Rect {
        Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(w, h))
    }

    #[test]
    fn wide_area_letterboxes_horizontally() {
        let t = ViewTransform::fit(rect(800.0, 300.0), (400, 300)).unwrap();
        assert_eq!(t.scale, 1.0);
        assert_eq!(t.image_rect.width(), 400.0);
        assert_eq!(t.image_rect.min.x, 10.0 + 200.0);
        assert_eq!(t.image_rect.min.y, 20.0);
    }

    #[test]
    fn scales_up_and_down_to_fit() {
        let up = ViewTransform::fit(rect(800.0, 600.0), (400, 300)).unwrap();
        assert_eq!(up.scale, 2.0);
        let down = ViewTransform::fit(rect(200.0, 150.0), (400, 300)).unwrap();
        assert_eq!(down.scale, 0.5);
    }

    #[test]
    fn round_trip() {
        let t = ViewTransform::fit(rect(640.0, 480.0), (1280, 720)).unwrap();
        let p = Point::new(321.0, 123.0);
        let back = t.to_image(t.to_screen(p));
        assert!((back.x - p.x).abs() < 1e-3 && (back.y - p.y).abs() < 1e-3);
    }

    #[test]
    fn clamps_to_the_image() {
        let t = ViewTransform::fit(rect(400.0, 300.0), (400, 300)).unwrap();
        let p = t.to_image_clamped(Pos2::new(-500.0, 9000.0));
        assert_eq!((p.x, p.y), (0.0, 300.0));
    }

    #[test]
    fn empty_sizes_give_none() {
        assert!(ViewTransform::fit(rect(0.0, 10.0), (4, 3)).is_none());
        assert!(ViewTransform::fit(rect(10.0, 10.0), (0, 3)).is_none());
    }
}

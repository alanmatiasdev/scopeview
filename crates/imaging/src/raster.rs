use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::RgbaImage;
use scopeview_core::overlay::{Primitive, Rgba as Color};
use scopeview_core::Point;

/// Padding around label text, in pixels.
const TEXT_PAD: f32 = 3.0;

fn font() -> &'static FontRef<'static> {
    use std::sync::OnceLock;
    static FONT: OnceLock<FontRef<'static>> = OnceLock::new();
    FONT.get_or_init(|| {
        FontRef::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT).expect("bundled font is valid")
    })
}

/// Draw `primitives` onto `image`.
pub fn render_overlay(image: &mut RgbaImage, primitives: &[Primitive]) {
    for primitive in primitives {
        match primitive {
            Primitive::Line { a, b, color, width } => draw_line(image, *a, *b, *color, *width),
            Primitive::RectOutline {
                min,
                max,
                color,
                width,
            } => draw_rect_outline(image, *min, *max, *color, *width),
            Primitive::Circle {
                center,
                radius,
                color,
                width,
            } => draw_circle(image, *center, *radius, *color, *width),
            Primitive::Handle {
                center,
                size,
                color,
            } => {
                let half = size / 2.0;
                let (x, y) = (center.x as f32, center.y as f32);
                fill_rect(image, x - half, y - half, x + half, y + half, *color);
            }
            Primitive::Text {
                pos,
                lines,
                color,
                size,
                background,
            } => draw_text(image, *pos, lines, *color, *size, *background),
        }
    }
}

/// Blend `color` over the pixel with extra `coverage` in 0..=1.
fn blend(image: &mut RgbaImage, x: i64, y: i64, color: Color, coverage: f32) {
    if x < 0 || y < 0 || x >= i64::from(image.width()) || y >= i64::from(image.height()) {
        return;
    }
    let alpha = (f32::from(color[3]) / 255.0) * coverage.clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let pixel = image.get_pixel_mut(x as u32, y as u32);
    for c in 0..3 {
        let old = f32::from(pixel[c]);
        pixel[c] = (old + (f32::from(color[c]) - old) * alpha).round() as u8;
    }
    pixel[3] = 255;
}

fn fill_rect(image: &mut RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32, color: Color) {
    for y in y0.floor() as i64..y1.ceil() as i64 {
        for x in x0.floor() as i64..x1.ceil() as i64 {
            blend(image, x, y, color, 1.0);
        }
    }
}

fn distance_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    (p.0 - (a.0 + t * dx)).hypot(p.1 - (a.1 + t * dy))
}

fn draw_line(image: &mut RgbaImage, a: Point, b: Point, color: Color, width: f32) {
    let half = width.max(1.0) / 2.0;
    let (a, b) = ((a.x as f32, a.y as f32), (b.x as f32, b.y as f32));
    let reach = half + 1.0;
    let x0 = (a.0.min(b.0) - reach).floor() as i64;
    let x1 = (a.0.max(b.0) + reach).ceil() as i64;
    let y0 = (a.1.min(b.1) - reach).floor() as i64;
    let y1 = (a.1.max(b.1) + reach).ceil() as i64;
    // Clip to the image so far-off shapes cost nothing.
    let (x0, x1) = (x0.max(0), x1.min(i64::from(image.width())));
    let (y0, y1) = (y0.max(0), y1.min(i64::from(image.height())));
    for y in y0..y1 {
        for x in x0..x1 {
            let d = distance_to_segment((x as f32 + 0.5, y as f32 + 0.5), a, b);
            blend(image, x, y, color, half + 0.5 - d);
        }
    }
}

fn draw_circle(image: &mut RgbaImage, center: Point, radius: f64, color: Color, width: f32) {
    let half = width.max(1.0) / 2.0;
    let (cx, cy, r) = (center.x as f32, center.y as f32, radius as f32);
    let reach = r + half + 1.0;
    let (x0, x1) = (
        ((cx - reach).floor() as i64).max(0),
        ((cx + reach).ceil() as i64).min(i64::from(image.width())),
    );
    let (y0, y1) = (
        ((cy - reach).floor() as i64).max(0),
        ((cy + reach).ceil() as i64).min(i64::from(image.height())),
    );
    for y in y0..y1 {
        for x in x0..x1 {
            let d = ((x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy) - r).abs();
            blend(image, x, y, color, half + 0.5 - d);
        }
    }
}

fn draw_rect_outline(image: &mut RgbaImage, min: Point, max: Point, color: Color, width: f32) {
    let (x0, y0, x1, y1) = (min.x as f32, min.y as f32, max.x as f32, max.y as f32);
    let w = width
        .max(1.0)
        .min((x1 - x0).max(1.0))
        .min((y1 - y0).max(1.0));
    fill_rect(image, x0, y0, x1, y0 + w, color); // top
    fill_rect(image, x0, y1 - w, x1, y1, color); // bottom
    fill_rect(image, x0, y0 + w, x0 + w, y1 - w, color); // left
    fill_rect(image, x1 - w, y0 + w, x1, y1 - w, color); // right
}

fn text_width<F: Font, S: ScaleFont<F>>(scaled: &S, text: &str) -> f32 {
    let mut width = 0.0;
    let mut previous = None;
    for ch in text.chars() {
        let id = scaled.glyph_id(ch);
        if let Some(p) = previous {
            width += scaled.kern(p, id);
        }
        width += scaled.h_advance(id);
        previous = Some(id);
    }
    width
}

fn draw_text(
    image: &mut RgbaImage,
    pos: Point,
    lines: &[String],
    color: Color,
    size: f32,
    background: Color,
) {
    let font = font();
    let scaled = font.as_scaled(PxScale::from(size));
    let line_height = scaled.height();
    let (left, mut top) = (pos.x as f32, pos.y as f32);
    for line in lines {
        let width = text_width(&scaled, line);
        fill_rect(
            image,
            left - TEXT_PAD,
            top - TEXT_PAD / 2.0,
            left + width + TEXT_PAD,
            top + line_height + TEXT_PAD / 2.0,
            background,
        );
        let baseline = top + scaled.ascent();
        let mut caret = left;
        let mut previous = None;
        for ch in line.chars() {
            let id = scaled.glyph_id(ch);
            if let Some(p) = previous {
                caret += scaled.kern(p, id);
            }
            let glyph =
                id.with_scale_and_position(scaled.scale(), ab_glyph::point(caret, baseline));
            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|gx, gy, coverage| {
                    let x = bounds.min.x as i64 + i64::from(gx);
                    let y = bounds.min.y as i64 + i64::from(gy);
                    blend(image, x, y, color, coverage);
                });
            }
            caret += scaled.h_advance(id);
            previous = Some(id);
        }
        top += line_height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn canvas() -> RgbaImage {
        RgbaImage::from_pixel(100, 60, Rgba([10, 20, 30, 255]))
    }

    const RED: Color = [255, 0, 0, 255];

    #[test]
    fn horizontal_line_colors_its_pixels_only() {
        let mut img = canvas();
        draw_line(
            &mut img,
            Point::new(10.0, 30.0),
            Point::new(90.0, 30.0),
            RED,
            2.0,
        );
        assert_eq!(img.get_pixel(50, 29).0, [255, 0, 0, 255]);
        assert_eq!(img.get_pixel(50, 30).0, [255, 0, 0, 255]);
        assert_eq!(img.get_pixel(50, 40).0, [10, 20, 30, 255]);
        assert_eq!(img.get_pixel(95, 30).0, [10, 20, 30, 255]);
    }

    #[test]
    fn circle_colors_the_ring_only() {
        let mut img = canvas();
        draw_circle(&mut img, Point::new(50.0, 30.0), 20.0, RED, 2.0);
        assert!(img.get_pixel(70, 30)[0] >= 250);
        assert!(img.get_pixel(30, 30)[0] >= 250);
        assert_eq!(img.get_pixel(50, 30).0, [10, 20, 30, 255]);
        assert_eq!(img.get_pixel(80, 30).0, [10, 20, 30, 255]);
    }

    #[test]
    fn handle_is_a_filled_square() {
        let mut img = canvas();
        render_overlay(
            &mut img,
            &[Primitive::Handle {
                center: Point::new(50.0, 30.0),
                size: 8.0,
                color: RED,
            }],
        );
        assert_eq!(img.get_pixel(50, 30).0, [255, 0, 0, 255]);
        assert_eq!(img.get_pixel(54, 30).0, [10, 20, 30, 255]);
    }

    #[test]
    fn rect_outline_leaves_the_inside_untouched() {
        let mut img = canvas();
        draw_rect_outline(
            &mut img,
            Point::new(20.0, 10.0),
            Point::new(80.0, 50.0),
            RED,
            2.0,
        );
        assert_eq!(img.get_pixel(20, 30).0, [255, 0, 0, 255]);
        assert_eq!(img.get_pixel(50, 10).0, [255, 0, 0, 255]);
        assert_eq!(img.get_pixel(79, 30).0, [255, 0, 0, 255]);
        assert_eq!(img.get_pixel(50, 30).0, [10, 20, 30, 255]);
        assert_eq!(img.get_pixel(5, 5).0, [10, 20, 30, 255]);
    }

    #[test]
    fn shapes_outside_the_image_do_not_panic() {
        let mut img = canvas();
        draw_line(
            &mut img,
            Point::new(-500.0, -500.0),
            Point::new(5000.0, 4000.0),
            RED,
            4.0,
        );
        draw_rect_outline(
            &mut img,
            Point::new(-10.0, -10.0),
            Point::new(500.0, 500.0),
            RED,
            3.0,
        );
        draw_text(
            &mut img,
            Point::new(95.0, 55.0),
            &["12.00mm".into()],
            RED,
            18.0,
            [0, 0, 0, 150],
        );
    }

    #[test]
    fn text_changes_pixels_near_its_anchor() {
        let mut img = canvas();
        draw_text(
            &mut img,
            Point::new(10.0, 10.0),
            &["8.00mm".into(), "S:1.00mm^2".into()],
            [255, 255, 255, 255],
            18.0,
            [0, 0, 0, 255],
        );
        let changed = img.pixels().filter(|p| p.0 != [10, 20, 30, 255]).count();
        assert!(changed > 100, "only {changed} pixels changed");
        assert_eq!(img.get_pixel(99, 0).0, [10, 20, 30, 255]);
    }

    #[test]
    fn render_overlay_draws_every_primitive() {
        let mut img = canvas();
        render_overlay(
            &mut img,
            &[Primitive::Line {
                a: Point::new(0.0, 5.0),
                b: Point::new(99.0, 5.0),
                color: RED,
                width: 2.0,
            }],
        );
        assert_eq!(img.get_pixel(50, 5).0, [255, 0, 0, 255]);
    }
}

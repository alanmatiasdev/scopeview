//! Image modes: grayscale, binarization, invert and emboss.
//!
//! Applied to each frame right after it is decoded (and flipped), so the preview, photos and
//! recordings all agree. Alpha is never touched. The weights, threshold and kernel are our own
//! choices (see `docs/specs/image-view.md`).

use image::RgbaImage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ImageMode {
    #[default]
    Normal,
    Gray,
    Binary,
    Invert,
    Emboss,
}

/// Luma at or above this becomes white in binary mode.
pub const BINARY_THRESHOLD: u8 = 128;

/// Emboss kernel, applied to luma. It sums to zero, so a flat area becomes mid gray.
const EMBOSS: [[i32; 3]; 3] = [[-2, -1, 0], [-1, 0, 1], [0, 1, 2]];

/// ITU-R BT.601 luma, integer math.
fn luma(r: u8, g: u8, b: u8) -> u8 {
    ((299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b)) / 1000) as u8
}

/// Redraw `image` in `mode`, in place.
pub fn apply_mode(image: &mut RgbaImage, mode: ImageMode) {
    match mode {
        ImageMode::Normal => {}
        ImageMode::Gray => per_pixel(image, |r, g, b| {
            let y = luma(r, g, b);
            [y, y, y]
        }),
        ImageMode::Binary => per_pixel(image, |r, g, b| {
            let v = if luma(r, g, b) >= BINARY_THRESHOLD {
                255
            } else {
                0
            };
            [v, v, v]
        }),
        ImageMode::Invert => per_pixel(image, |r, g, b| [255 - r, 255 - g, 255 - b]),
        ImageMode::Emboss => emboss(image),
    }
}

fn per_pixel(image: &mut RgbaImage, f: impl Fn(u8, u8, u8) -> [u8; 3]) {
    for px in image.pixels_mut() {
        let [r, g, b] = f(px.0[0], px.0[1], px.0[2]);
        px.0[0] = r;
        px.0[1] = g;
        px.0[2] = b;
    }
}

fn emboss(image: &mut RgbaImage) {
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 {
        return;
    }
    let lum: Vec<u8> = image
        .pixels()
        .map(|p| luma(p.0[0], p.0[1], p.0[2]))
        .collect();
    let at = |x: i64, y: i64| -> i32 {
        let x = x.clamp(0, i64::from(w) - 1) as usize;
        let y = y.clamp(0, i64::from(h) - 1) as usize;
        i32::from(lum[y * w as usize + x])
    };
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0;
            for (ky, row) in EMBOSS.iter().enumerate() {
                for (kx, k) in row.iter().enumerate() {
                    sum += k * at(i64::from(x) + kx as i64 - 1, i64::from(y) + ky as i64 - 1);
                }
            }
            let v = (sum + 128).clamp(0, 255) as u8;
            let px = image.get_pixel_mut(x, y);
            px.0[0] = v;
            px.0[1] = v;
            px.0[2] = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn one(px: [u8; 4], mode: ImageMode) -> [u8; 4] {
        let mut img = RgbaImage::from_pixel(1, 1, Rgba(px));
        apply_mode(&mut img, mode);
        img.get_pixel(0, 0).0
    }

    #[test]
    fn normal_changes_nothing() {
        assert_eq!(one([10, 20, 30, 40], ImageMode::Normal), [10, 20, 30, 40]);
    }

    #[test]
    fn gray_uses_luma_weights_and_keeps_alpha() {
        assert_eq!(one([255, 0, 0, 7], ImageMode::Gray), [76, 76, 76, 7]);
        assert_eq!(one([0, 255, 0, 255], ImageMode::Gray), [149, 149, 149, 255]);
        assert_eq!(one([0, 0, 255, 255], ImageMode::Gray), [29, 29, 29, 255]);
        assert_eq!(
            one([255, 255, 255, 255], ImageMode::Gray),
            [255, 255, 255, 255]
        );
    }

    #[test]
    fn binary_splits_at_the_threshold() {
        assert_eq!(one([127, 127, 127, 9], ImageMode::Binary), [0, 0, 0, 9]);
        assert_eq!(
            one([128, 128, 128, 9], ImageMode::Binary),
            [255, 255, 255, 9]
        );
        assert_eq!(one([255, 0, 0, 255], ImageMode::Binary), [0, 0, 0, 255]);
    }

    #[test]
    fn invert_flips_color_not_alpha() {
        assert_eq!(one([0, 100, 255, 33], ImageMode::Invert), [255, 155, 0, 33]);
        let mut img = RgbaImage::from_pixel(2, 2, Rgba([1, 2, 3, 4]));
        apply_mode(&mut img, ImageMode::Invert);
        apply_mode(&mut img, ImageMode::Invert);
        assert_eq!(img.get_pixel(1, 1).0, [1, 2, 3, 4]);
    }

    #[test]
    fn emboss_of_a_flat_image_is_mid_gray() {
        let mut img = RgbaImage::from_pixel(4, 4, Rgba([200, 10, 90, 255]));
        apply_mode(&mut img, ImageMode::Emboss);
        assert!(img.pixels().all(|p| p.0 == [128, 128, 128, 255]));
    }

    #[test]
    fn emboss_responds_to_an_edge_by_direction() {
        // Dark left half, bright right half: a vertical edge between x=1 and x=2.
        let mut img = RgbaImage::from_fn(4, 3, |x, _| {
            let v = if x < 2 { 0 } else { 200 };
            Rgba([v, v, v, 255])
        });
        apply_mode(&mut img, ImageMode::Emboss);
        // Interior row, either side of the edge: brighter than mid gray (light from the edge).
        assert!(img.get_pixel(1, 1).0[0] > 128);
        assert!(img.get_pixel(2, 1).0[0] > 128);
        // Away from the edge it is flat.
        assert_eq!(img.get_pixel(0, 1).0[0], 128);
        assert_eq!(img.get_pixel(3, 1).0[0], 128);
    }

    #[test]
    fn empty_image_is_fine() {
        let mut img = RgbaImage::new(0, 0);
        apply_mode(&mut img, ImageMode::Emboss);
    }
}

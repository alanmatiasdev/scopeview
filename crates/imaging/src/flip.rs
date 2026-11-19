use image::imageops;
use image::RgbaImage;

/// Mirror `image` left-right (`horizontal`) and/or top-bottom (`vertical`) in place.
pub fn flip(image: &mut RgbaImage, horizontal: bool, vertical: bool) {
    if horizontal {
        imageops::flip_horizontal_in_place(image);
    }
    if vertical {
        imageops::flip_vertical_in_place(image);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn sample() -> RgbaImage {
        // 3x2: pixel value encodes its position.
        RgbaImage::from_fn(3, 2, |x, y| Rgba([x as u8, y as u8, 0, 255]))
    }

    #[test]
    fn no_flip_changes_nothing() {
        let mut img = sample();
        flip(&mut img, false, false);
        assert_eq!(img, sample());
    }

    #[test]
    fn horizontal_mirrors_columns() {
        let mut img = sample();
        flip(&mut img, true, false);
        assert_eq!(img.get_pixel(0, 0).0, [2, 0, 0, 255]);
        assert_eq!(img.get_pixel(2, 1).0, [0, 1, 0, 255]);
    }

    #[test]
    fn vertical_mirrors_rows() {
        let mut img = sample();
        flip(&mut img, false, true);
        assert_eq!(img.get_pixel(0, 0).0, [0, 1, 0, 255]);
        assert_eq!(img.get_pixel(1, 1).0, [1, 0, 0, 255]);
    }

    #[test]
    fn both_is_a_half_turn_and_twice_restores() {
        let mut img = sample();
        flip(&mut img, true, true);
        assert_eq!(img.get_pixel(0, 0).0, [2, 1, 0, 255]);
        flip(&mut img, true, true);
        assert_eq!(img, sample());
    }
}

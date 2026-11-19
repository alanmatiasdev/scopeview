//! Small previews of saved photos and videos, kept in a `Thumbnails` folder next to them.

use std::fs;
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::{imageops, DynamicImage, RgbaImage};
use scopeview_core::recent::FileKind;

/// Longest side of a thumbnail, in pixels (our own choice).
pub const THUMB_SIDE: u32 = 128;

const THUMB_QUALITY: u8 = 80;
const THUMB_FOLDER: &str = "Thumbnails";

/// `image` scaled down to fit in `max_side` x `max_side`, keeping the aspect ratio. Never scaled up.
pub fn thumbnail(image: &RgbaImage, max_side: u32) -> RgbaImage {
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 || max_side == 0 {
        return RgbaImage::new(0, 0);
    }
    if w <= max_side && h <= max_side {
        return image.clone();
    }
    let scale = f64::from(max_side) / f64::from(w.max(h));
    let tw = ((f64::from(w) * scale).round() as u32).max(1);
    let th = ((f64::from(h) * scale).round() as u32).max(1);
    imageops::thumbnail(image, tw, th)
}

/// Where the thumbnail of `file` lives: `<folder of file>/Thumbnails/<name without extension>.jpg`.
pub fn thumbnail_path(file: &Path) -> Option<PathBuf> {
    let stem = file.file_stem()?;
    let dir = file.parent()?;
    Some(dir.join(THUMB_FOLDER).join(stem).with_extension("jpg"))
}

/// Write the thumbnail for `file` from `image` (any size; it is shrunk). Returns where it went.
pub fn write_thumbnail(image: &RgbaImage, file: &Path) -> std::io::Result<PathBuf> {
    let path = thumbnail_path(file)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no file name"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let small = thumbnail(image, THUMB_SIDE);
    let rgb = DynamicImage::ImageRgba8(small).to_rgb8();
    let out = fs::File::create(&path)?;
    let mut writer = std::io::BufWriter::new(out);
    JpegEncoder::new_with_quality(&mut writer, THUMB_QUALITY)
        .encode_image(&rgb)
        .map_err(std::io::Error::other)?;
    Ok(path)
}

/// The thumbnail of `file`: the saved one, or, for a photo without one, made now from the photo
/// and saved for next time. `None` for a video without a thumbnail or a file that cannot be read.
pub fn load_thumbnail(file: &Path) -> Option<RgbaImage> {
    if let Some(saved) = thumbnail_path(file).and_then(|p| image::open(p).ok()) {
        return Some(saved.to_rgba8());
    }
    if FileKind::of(file) == FileKind::Video {
        return None;
    }
    let full = image::open(file).ok()?.to_rgba8();
    let small = thumbnail(&full, THUMB_SIDE);
    let _ = write_thumbnail(&small, file);
    Some(small)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("scopeview-thumb-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn shrinks_to_fit_and_keeps_the_aspect_ratio() {
        let big = RgbaImage::from_pixel(1280, 720, Rgba([9, 9, 9, 255]));
        assert_eq!(thumbnail(&big, 128).dimensions(), (128, 72));
        let tall = RgbaImage::from_pixel(100, 400, Rgba([9, 9, 9, 255]));
        assert_eq!(thumbnail(&tall, 128).dimensions(), (32, 128));
    }

    #[test]
    fn small_images_are_not_scaled_up() {
        let small = RgbaImage::from_pixel(40, 30, Rgba([1, 2, 3, 255]));
        assert_eq!(thumbnail(&small, 128), small);
    }

    #[test]
    fn degenerate_inputs_give_an_empty_image() {
        assert_eq!(thumbnail(&RgbaImage::new(0, 0), 128).dimensions(), (0, 0));
        let img = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 255]));
        assert_eq!(thumbnail(&img, 0).dimensions(), (0, 0));
    }

    #[test]
    fn path_is_next_to_the_file() {
        assert_eq!(
            thumbnail_path(Path::new("/p/scopeview-1.png")).unwrap(),
            PathBuf::from("/p/Thumbnails/scopeview-1.jpg")
        );
        assert_eq!(
            thumbnail_path(Path::new("/p/clip.avi")).unwrap(),
            PathBuf::from("/p/Thumbnails/clip.jpg")
        );
        assert!(thumbnail_path(Path::new("/")).is_none());
    }

    #[test]
    fn written_thumbnail_can_be_read_back() {
        let dir = temp_dir("write");
        let file = dir.join("shot.png");
        let img = RgbaImage::from_pixel(640, 480, Rgba([200, 30, 30, 255]));
        let written = write_thumbnail(&img, &file).unwrap();
        assert_eq!(written, dir.join("Thumbnails").join("shot.jpg"));
        let back = load_thumbnail(&file).unwrap();
        assert_eq!(back.dimensions(), (128, 96));
        assert!(back.get_pixel(10, 10).0[0] > 150);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_photo_without_a_thumbnail_gets_one_made() {
        let dir = temp_dir("make");
        let file = dir.join("photo.png");
        RgbaImage::from_pixel(300, 200, Rgba([10, 200, 10, 255]))
            .save(&file)
            .unwrap();
        assert!(!dir.join("Thumbnails").exists());
        let t = load_thumbnail(&file).unwrap();
        assert_eq!(t.dimensions(), (128, 85));
        assert!(dir.join("Thumbnails").join("photo.jpg").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn videos_without_a_thumbnail_and_missing_files_give_none() {
        let dir = temp_dir("none");
        let video = dir.join("clip.avi");
        fs::write(&video, b"not really").unwrap();
        assert!(load_thumbnail(&video).is_none());
        assert!(load_thumbnail(&dir.join("gone.jpg")).is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}

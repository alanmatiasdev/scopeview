use std::fs;
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat, RgbaImage};
use scopeview_core::overlay::Primitive;
use scopeview_core::PhotoFormat;

use crate::{render_overlay, thumbnail, write_thumbnail, THUMB_SIDE};

/// JPEG quality used for photos (our own choice; 1..=100).
pub const JPEG_QUALITY: u8 = 90;

#[derive(Debug, thiserror::Error)]
pub enum PhotoError {
    #[error("could not create folder {0}: {1}")]
    Folder(PathBuf, std::io::Error),
    #[error("could not write {0}: {1}")]
    Write(PathBuf, String),
}

/// Burn `primitives` into a copy of `frame` and save it as `<dir>/scopeview-<stamp>.<ext>`.
///
/// The folder is created when missing. If the name is taken, `-2`, `-3`, … is appended.
/// The saved image has the frame's own resolution. Returns the path written.
pub fn save_photo(
    frame: &RgbaImage,
    primitives: &[Primitive],
    dir: &Path,
    format: PhotoFormat,
    stamp: &str,
) -> Result<PathBuf, PhotoError> {
    fs::create_dir_all(dir).map_err(|e| PhotoError::Folder(dir.to_path_buf(), e))?;
    let mut image = frame.clone();
    render_overlay(&mut image, primitives);

    // Shrunk now, while the burned-in image is still around; written once the photo is saved.
    let thumb = thumbnail(&image, THUMB_SIDE);

    let ext = format.extension();
    let mut path = dir.join(format!("scopeview-{stamp}.{ext}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("scopeview-{stamp}-{n}.{ext}"));
        n += 1;
    }
    let result = match format {
        PhotoFormat::Jpeg => fs::File::create(&path)
            .map_err(|e| e.to_string())
            .and_then(|file| {
                let rgb = DynamicImage::ImageRgba8(image).to_rgb8();
                let mut writer = std::io::BufWriter::new(file);
                JpegEncoder::new_with_quality(&mut writer, JPEG_QUALITY)
                    .encode_image(&rgb)
                    .map_err(|e| e.to_string())
            }),
        PhotoFormat::Png => image
            .save_with_format(&path, ImageFormat::Png)
            .map_err(|e| e.to_string()),
        PhotoFormat::Bmp => DynamicImage::ImageRgba8(image)
            .to_rgb8()
            .save_with_format(&path, ImageFormat::Bmp)
            .map_err(|e| e.to_string()),
    };
    match result {
        Ok(()) => {
            // A missing thumbnail only costs a slower list later, so a failure here is ignored.
            let _ = write_thumbnail(&thumb, &path);
            Ok(path)
        }
        Err(message) => {
            let _ = fs::remove_file(&path);
            Err(PhotoError::Write(path, message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use scopeview_core::Point;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("scopeview-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn line() -> Vec<Primitive> {
        vec![Primitive::Line {
            a: Point::new(0.0, 10.0),
            b: Point::new(63.0, 10.0),
            color: [255, 0, 0, 255],
            width: 4.0,
        }]
    }

    #[test]
    fn png_keeps_resolution_and_burns_overlay() {
        let dir = temp_dir("png");
        let frame = RgbaImage::from_pixel(64, 48, Rgba([0, 0, 255, 255]));
        let path = save_photo(&frame, &line(), &dir, PhotoFormat::Png, "t1").unwrap();
        assert_eq!(path.file_name().unwrap(), "scopeview-t1.png");
        let saved = image::open(&path).unwrap().to_rgba8();
        assert_eq!(saved.dimensions(), (64, 48));
        assert_eq!(saved.get_pixel(30, 10).0, [255, 0, 0, 255]);
        assert_eq!(saved.get_pixel(30, 40).0, [0, 0, 255, 255]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn jpeg_keeps_resolution_and_burns_overlay() {
        let dir = temp_dir("jpg");
        let frame = RgbaImage::from_pixel(64, 48, Rgba([0, 0, 255, 255]));
        let path = save_photo(
            &frame,
            &line(),
            &dir.join("nested"),
            PhotoFormat::Jpeg,
            "t2",
        )
        .unwrap();
        assert_eq!(path.extension().unwrap(), "jpg");
        let saved = image::open(&path).unwrap().to_rgb8();
        assert_eq!(saved.dimensions(), (64, 48));
        let p = saved.get_pixel(30, 10).0;
        assert!(p[0] > 200 && p[2] < 80, "line pixel was {p:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_a_photo_also_writes_its_thumbnail() {
        let dir = temp_dir("thumb");
        let frame = RgbaImage::from_pixel(640, 480, Rgba([0, 0, 255, 255]));
        let path = save_photo(&frame, &line(), &dir, PhotoFormat::Png, "t4").unwrap();
        let thumb = dir.join("Thumbnails").join("scopeview-t4.jpg");
        assert!(thumb.exists(), "expected {}", thumb.display());
        assert_eq!(image::open(&thumb).unwrap().width(), 128);
        assert!(path.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn bmp_keeps_resolution_and_burns_overlay() {
        let dir = temp_dir("bmp");
        let frame = RgbaImage::from_pixel(64, 48, Rgba([0, 0, 255, 255]));
        let path = save_photo(&frame, &line(), &dir, PhotoFormat::Bmp, "t3").unwrap();
        assert_eq!(path.extension().unwrap(), "bmp");
        let saved = image::open(&path).unwrap().to_rgb8();
        assert_eq!(saved.dimensions(), (64, 48));
        assert_eq!(saved.get_pixel(30, 10).0, [255, 0, 0]);
        assert_eq!(saved.get_pixel(30, 40).0, [0, 0, 255]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn taken_names_get_a_suffix_and_the_input_frame_is_untouched() {
        let dir = temp_dir("dup");
        let frame = RgbaImage::from_pixel(8, 8, Rgba([1, 2, 3, 255]));
        let a = save_photo(&frame, &line(), &dir, PhotoFormat::Png, "same").unwrap();
        let b = save_photo(&frame, &line(), &dir, PhotoFormat::Png, "same").unwrap();
        assert_ne!(a, b);
        assert_eq!(b.file_name().unwrap(), "scopeview-same-2.png");
        assert_eq!(frame.get_pixel(0, 4).0, [1, 2, 3, 255]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unwritable_folder_is_an_error() {
        let dir = temp_dir("blocked");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("file");
        fs::write(&file, b"x").unwrap();
        let frame = RgbaImage::new(4, 4);
        let err = save_photo(&frame, &[], &file.join("sub"), PhotoFormat::Png, "x");
        assert!(matches!(err, Err(PhotoError::Folder(..))));
        let _ = fs::remove_dir_all(&dir);
    }
}

use std::fmt;

use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{
    ApiBackend, CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType,
};
use nokhwa::Camera;

use crate::CameraError;

/// Backend-specific device index. Not stable across unplug/replug, so match by name too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelKind {
    Mjpeg,
    Yuyv,
    Nv12,
    Gray,
    Raw,
}

impl fmt::Display for PixelKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mjpeg => "MJPEG",
            Self::Yuyv => "YUYV",
            Self::Nv12 => "NV12",
            Self::Gray => "GRAY",
            Self::Raw => "RAW",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FormatInfo {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub pixel: PixelKind,
}

impl fmt::Display for FormatInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}x{} @ {} fps ({})",
            self.width, self.height, self.fps, self.pixel
        )
    }
}

impl FormatInfo {
    /// Larger resolution first, then higher frame rate, then MJPEG before raw formats.
    fn sort_key(&self) -> (std::cmp::Reverse<u64>, std::cmp::Reverse<u32>, u8) {
        let area = u64::from(self.width) * u64::from(self.height);
        let pixel_rank = match self.pixel {
            PixelKind::Mjpeg => 0,
            _ => 1,
        };
        (
            std::cmp::Reverse(area),
            std::cmp::Reverse(self.fps),
            pixel_rank,
        )
    }

    pub(crate) fn to_backend(self) -> CameraFormat {
        CameraFormat::new_from(
            self.width,
            self.height,
            match self.pixel {
                PixelKind::Mjpeg => FrameFormat::MJPEG,
                PixelKind::Yuyv => FrameFormat::YUYV,
                PixelKind::Nv12 => FrameFormat::NV12,
                PixelKind::Gray => FrameFormat::GRAY,
                PixelKind::Raw => FrameFormat::RAWRGB,
            },
            self.fps,
        )
    }

    pub(crate) fn from_backend(format: CameraFormat) -> Self {
        Self {
            width: format.width(),
            height: format.height(),
            fps: format.frame_rate(),
            pixel: match format.format() {
                FrameFormat::MJPEG => PixelKind::Mjpeg,
                FrameFormat::YUYV => PixelKind::Yuyv,
                FrameFormat::NV12 => PixelKind::Nv12,
                FrameFormat::GRAY => PixelKind::Gray,
                FrameFormat::RAWRGB | FrameFormat::RAWBGR => PixelKind::Raw,
            },
        }
    }
}

/// Sort formats for display and drop exact duplicates.
pub(crate) fn sorted_unique(mut formats: Vec<FormatInfo>) -> Vec<FormatInfo> {
    formats.sort_by_key(FormatInfo::sort_key);
    formats.dedup();
    formats
}

/// Which kind of pixel format the automatic choice favors when sizes and rates are equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelPreference {
    /// Compressed MJPEG: more frames over USB 2.0, decoded on our side.
    #[default]
    Mjpeg,
    /// Uncompressed formats (YUYV, NV12, ...): no compression artifacts, lower frame rates.
    Raw,
}

/// Pick the format to use when the user has not chosen one, from what the device really offers.
///
/// Prefers a usable frame rate (15 fps or more), then the resolution closest to 1280x720, then
/// the preferred pixel kind (MJPEG by default, raw formats with `PixelPreference::Raw`), then the
/// higher frame rate. Never returns a format that is not in
/// `formats`, so devices that only offer, say, 640x480 YUYV (common for USB microscopes) work.
pub(crate) fn choose_default_format(
    formats: &[FormatInfo],
    preference: PixelPreference,
) -> Option<FormatInfo> {
    const TARGET_AREA: i64 = 1280 * 720;
    formats.iter().copied().min_by_key(|f| {
        let area = i64::from(f.width) * i64::from(f.height);
        let is_mjpeg = f.pixel == PixelKind::Mjpeg;
        (
            f.fps < 15,
            (area - TARGET_AREA).abs(),
            is_mjpeg != (preference == PixelPreference::Mjpeg),
            std::cmp::Reverse(f.fps),
        )
    })
}

/// After an unplug, find the same physical device again: same name at the same index first,
/// then same name at any index. Never fall back to a different device.
pub(crate) fn resolve_device(
    devices: &[DeviceInfo],
    name: &str,
    last_id: DeviceId,
) -> Option<DeviceInfo> {
    devices
        .iter()
        .find(|d| d.name == name && d.id == last_id)
        .or_else(|| devices.iter().find(|d| d.name == name))
        .cloned()
}

/// Ask the OS for camera access. Only macOS prompts; elsewhere this returns immediately.
pub fn init_permissions() -> Result<(), CameraError> {
    #[cfg(target_os = "macos")]
    {
        use std::sync::mpsc;
        let (tx, rx) = mpsc::channel();
        nokhwa::nokhwa_initialize(move |granted| {
            let _ = tx.send(granted);
        });
        match rx.recv() {
            Ok(true) => Ok(()),
            _ => Err(CameraError::PermissionDenied),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

pub fn list_devices() -> Result<Vec<DeviceInfo>, CameraError> {
    let found = nokhwa::query(ApiBackend::Auto)?;
    Ok(found
        .into_iter()
        .filter_map(|info| match info.index() {
            CameraIndex::Index(i) if is_capture_node(*i) => Some(DeviceInfo {
                id: DeviceId(*i),
                name: info.human_name(),
            }),
            _ => None,
        })
        .collect())
}

/// UVC cameras on Linux expose a second `/dev/videoN` node for metadata. It has no video formats
/// and shows up as a duplicate device, so keep only the node with sysfs `index` 0.
#[cfg(target_os = "linux")]
fn is_capture_node(id: u32) -> bool {
    std::fs::read_to_string(format!("/sys/class/video4linux/video{id}/index"))
        .map_or(true, |index| index.trim() == "0")
}

#[cfg(not(target_os = "linux"))]
fn is_capture_node(_id: u32) -> bool {
    true
}

pub fn list_formats(device: &DeviceInfo) -> Result<Vec<FormatInfo>, CameraError> {
    let mut camera = Camera::new(
        CameraIndex::Index(device.id.0),
        RequestedFormat::new::<RgbAFormat>(RequestedFormatType::None),
    )?;
    let formats = camera
        .compatible_camera_formats()?
        .into_iter()
        .map(FormatInfo::from_backend)
        .collect();
    Ok(sorted_unique(formats))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(id: u32, name: &str) -> DeviceInfo {
        DeviceInfo {
            id: DeviceId(id),
            name: name.to_string(),
        }
    }

    fn fmt(width: u32, height: u32, fps: u32, pixel: PixelKind) -> FormatInfo {
        FormatInfo {
            width,
            height,
            fps,
            pixel,
        }
    }

    #[test]
    fn resolve_prefers_same_name_and_index() {
        let devices = [dev(0, "Cam A"), dev(1, "Cam B"), dev(2, "Cam B")];
        assert_eq!(
            resolve_device(&devices, "Cam B", DeviceId(2)),
            Some(dev(2, "Cam B"))
        );
    }

    #[test]
    fn resolve_follows_device_to_new_index() {
        let devices = [dev(0, "Cam A"), dev(3, "Cam B")];
        assert_eq!(
            resolve_device(&devices, "Cam B", DeviceId(1)),
            Some(dev(3, "Cam B"))
        );
    }

    #[test]
    fn resolve_never_picks_a_different_device() {
        let devices = [dev(0, "Cam A")];
        assert_eq!(resolve_device(&devices, "Cam B", DeviceId(0)), None);
    }

    #[test]
    fn formats_sort_large_first_then_fast_then_mjpeg() {
        let sorted = sorted_unique(vec![
            fmt(640, 480, 30, PixelKind::Yuyv),
            fmt(1920, 1080, 5, PixelKind::Yuyv),
            fmt(1920, 1080, 30, PixelKind::Mjpeg),
            fmt(1920, 1080, 30, PixelKind::Yuyv),
            fmt(1920, 1080, 30, PixelKind::Mjpeg),
        ]);
        assert_eq!(
            sorted,
            vec![
                fmt(1920, 1080, 30, PixelKind::Mjpeg),
                fmt(1920, 1080, 30, PixelKind::Yuyv),
                fmt(1920, 1080, 5, PixelKind::Yuyv),
                fmt(640, 480, 30, PixelKind::Yuyv),
            ]
        );
    }

    #[test]
    fn default_format_is_closest_to_720p_mjpeg() {
        let formats = [
            fmt(1920, 1080, 30, PixelKind::Mjpeg),
            fmt(1280, 720, 10, PixelKind::Yuyv),
            fmt(1280, 720, 30, PixelKind::Mjpeg),
            fmt(640, 480, 30, PixelKind::Mjpeg),
        ];
        assert_eq!(
            choose_default_format(&formats, PixelPreference::Mjpeg),
            Some(fmt(1280, 720, 30, PixelKind::Mjpeg))
        );
    }

    #[test]
    fn default_format_works_for_yuyv_only_microscope() {
        let formats = [
            fmt(640, 480, 5, PixelKind::Yuyv),
            fmt(640, 480, 30, PixelKind::Yuyv),
        ];
        assert_eq!(
            choose_default_format(&formats, PixelPreference::Mjpeg),
            Some(fmt(640, 480, 30, PixelKind::Yuyv))
        );
    }

    #[test]
    fn default_format_avoids_slideshow_rates_when_possible() {
        let formats = [
            fmt(1280, 720, 5, PixelKind::Mjpeg),
            fmt(800, 600, 30, PixelKind::Yuyv),
        ];
        assert_eq!(
            choose_default_format(&formats, PixelPreference::Mjpeg),
            Some(fmt(800, 600, 30, PixelKind::Yuyv))
        );
    }

    #[test]
    fn raw_preference_picks_uncompressed_at_the_same_size_and_rate() {
        let formats = [
            fmt(1280, 720, 30, PixelKind::Mjpeg),
            fmt(1280, 720, 30, PixelKind::Yuyv),
        ];
        assert_eq!(
            choose_default_format(&formats, PixelPreference::Raw),
            Some(fmt(1280, 720, 30, PixelKind::Yuyv))
        );
        assert_eq!(
            choose_default_format(&formats, PixelPreference::Mjpeg),
            Some(fmt(1280, 720, 30, PixelKind::Mjpeg))
        );
    }

    #[test]
    fn preference_never_beats_size_or_usable_rate() {
        let formats = [
            fmt(1280, 720, 30, PixelKind::Mjpeg),
            fmt(1280, 720, 5, PixelKind::Yuyv),
            fmt(640, 480, 30, PixelKind::Yuyv),
        ];
        // Raw is preferred, but the only 720p raw mode is a slideshow and 640x480 is further
        // from the target than 720p MJPEG.
        assert_eq!(
            choose_default_format(&formats, PixelPreference::Raw),
            Some(fmt(1280, 720, 30, PixelKind::Mjpeg))
        );
    }

    #[test]
    fn preference_falls_back_when_only_one_kind_exists() {
        let mjpeg_only = [fmt(640, 480, 30, PixelKind::Mjpeg)];
        let raw_only = [fmt(640, 480, 30, PixelKind::Yuyv)];
        assert!(choose_default_format(&mjpeg_only, PixelPreference::Raw).is_some());
        assert!(choose_default_format(&raw_only, PixelPreference::Mjpeg).is_some());
    }

    #[test]
    fn default_format_of_nothing_is_nothing() {
        assert_eq!(choose_default_format(&[], PixelPreference::Mjpeg), None);
    }

    #[test]
    fn format_display_is_human_readable() {
        assert_eq!(
            fmt(1280, 720, 30, PixelKind::Mjpeg).to_string(),
            "1280x720 @ 30 fps (MJPEG)"
        );
    }
}

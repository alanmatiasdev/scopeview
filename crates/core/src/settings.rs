//! User settings, as a plain value with defaults. Reading and writing files is the app's job.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PhotoFormat {
    Jpeg,
    Png,
    Bmp,
}

impl PhotoFormat {
    pub fn extension(self) -> &'static str {
        match self {
            PhotoFormat::Jpeg => "jpg",
            PhotoFormat::Png => "png",
            PhotoFormat::Bmp => "bmp",
        }
    }
}

/// Color scheme of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the operating system.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Folder for photos. Empty means "use the default folder" (chosen by the app).
    pub save_dir: String,
    pub line_color: [u8; 3],
    /// Stroke width in image pixels.
    pub line_width: u32,
    pub text_color: [u8; 3],
    /// Label text height in image pixels.
    pub text_size: u32,
    /// Color of the crosshair and its ruler ticks.
    pub grid_color: [u8; 3],
    pub photo_format: PhotoFormat,
    /// A recording stops by itself after this many seconds. 0 means no limit.
    pub max_record_seconds: u32,
    /// Name of the camera used last time.
    pub last_device: Option<String>,
    /// UI language code such as `pt-BR`. `None` follows the operating system.
    pub language: Option<String>,
    /// With the Automatic format, choose MJPEG over raw formats when both fit.
    pub prefer_mjpeg: bool,
    /// Filter the image when it is shown larger or smaller than the camera's pixels.
    pub smooth_scaling: bool,
    pub theme: Theme,
    /// Stronger colors and outlines for the interface.
    pub high_contrast: bool,
    /// Interface zoom, 1.0 is the normal size.
    pub ui_scale: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            save_dir: String::new(),
            line_color: [255, 64, 64],
            line_width: 2,
            text_color: [255, 255, 255],
            text_size: 18,
            grid_color: [255, 220, 64],
            photo_format: PhotoFormat::Jpeg,
            max_record_seconds: 0,
            last_device: None,
            language: None,
            prefer_mjpeg: true,
            smooth_scaling: true,
            theme: Theme::System,
            high_contrast: false,
            ui_scale: 1.0,
        }
    }
}

pub const MAX_LINE_WIDTH: u32 = 10;
pub const MIN_TEXT_SIZE: u32 = 8;
pub const MAX_TEXT_SIZE: u32 = 72;
pub const MIN_UI_SCALE: f32 = 0.5;
pub const MAX_UI_SCALE: f32 = 3.0;
/// The interface sizes the settings window offers.
pub const UI_SCALE_PRESETS: [f32; 7] = [0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 3.0];
/// Longest recording time limit: 8 hours.
pub const MAX_RECORD_SECONDS: u32 = 28_800;
/// The limits the settings window offers, in seconds. 0 is "no limit".
pub const RECORD_LIMIT_PRESETS: [u32; 11] = [
    0,
    10,
    30,
    60,
    300,
    600,
    1_800,
    3_600,
    7_200,
    14_400,
    MAX_RECORD_SECONDS,
];

impl Settings {
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("settings always serialize")
    }

    /// Parse a saved file; a broken one gives defaults and a warning. Values are clamped to range.
    pub fn from_toml_lossy(text: &str) -> (Self, Option<String>) {
        match toml::from_str::<Settings>(text) {
            Ok(mut s) => {
                s.line_width = s.line_width.clamp(1, MAX_LINE_WIDTH);
                s.text_size = s.text_size.clamp(MIN_TEXT_SIZE, MAX_TEXT_SIZE);
                s.max_record_seconds = s.max_record_seconds.min(MAX_RECORD_SECONDS);
                s.ui_scale = if s.ui_scale.is_finite() {
                    s.ui_scale.clamp(MIN_UI_SCALE, MAX_UI_SCALE)
                } else {
                    1.0
                };
                (s, None)
            }
            Err(error) => (
                Self::default(),
                Some(format!("settings file ignored: {error}")),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let s = Settings {
            save_dir: "/tmp/photos".into(),
            line_color: [1, 2, 3],
            line_width: 4,
            text_color: [9, 8, 7],
            text_size: 30,
            grid_color: [4, 5, 6],
            photo_format: PhotoFormat::Png,
            max_record_seconds: 600,
            last_device: Some("Microscope".into()),
            language: Some("pt-BR".into()),
            prefer_mjpeg: false,
            smooth_scaling: false,
            theme: Theme::Dark,
            high_contrast: true,
            ui_scale: 1.5,
        };
        let (loaded, warning) = Settings::from_toml_lossy(&s.to_toml());
        assert_eq!(warning, None);
        assert_eq!(loaded, s);
    }

    #[test]
    fn missing_keys_use_defaults() {
        let (s, warning) = Settings::from_toml_lossy("line_width = 3");
        assert_eq!(warning, None);
        assert_eq!(s.line_width, 3);
        assert_eq!(s.photo_format, PhotoFormat::Jpeg);
    }

    #[test]
    fn broken_file_gives_defaults_and_warning() {
        let (s, warning) = Settings::from_toml_lossy("photo_format = 12 [");
        assert_eq!(s, Settings::default());
        assert!(warning.is_some());
    }

    #[test]
    fn width_is_clamped() {
        assert_eq!(Settings::from_toml_lossy("line_width = 0").0.line_width, 1);
        assert_eq!(
            Settings::from_toml_lossy("line_width = 99").0.line_width,
            MAX_LINE_WIDTH
        );
    }

    #[test]
    fn text_size_is_clamped() {
        assert_eq!(
            Settings::from_toml_lossy("text_size = 1").0.text_size,
            MIN_TEXT_SIZE
        );
        assert_eq!(
            Settings::from_toml_lossy("text_size = 500").0.text_size,
            MAX_TEXT_SIZE
        );
    }

    #[test]
    fn record_limit_is_capped_and_defaults_to_none() {
        assert_eq!(Settings::default().max_record_seconds, 0);
        assert_eq!(
            Settings::from_toml_lossy("max_record_seconds = 999999")
                .0
                .max_record_seconds,
            MAX_RECORD_SECONDS
        );
        assert_eq!(RECORD_LIMIT_PRESETS[0], 0);
        assert!(RECORD_LIMIT_PRESETS.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn new_settings_have_defaults_and_old_files_still_load() {
        let d = Settings::default();
        assert_eq!(d.language, None);
        assert!(d.prefer_mjpeg && d.smooth_scaling && !d.high_contrast);
        assert_eq!((d.theme, d.ui_scale), (Theme::System, 1.0));
        let (s, warning) = Settings::from_toml_lossy("line_width = 3\nphoto_format = \"png\"");
        assert_eq!(warning, None);
        assert_eq!(s.language, None);
        assert!(s.prefer_mjpeg);
    }

    #[test]
    fn ui_scale_is_clamped() {
        assert_eq!(
            Settings::from_toml_lossy("ui_scale = 0.01").0.ui_scale,
            MIN_UI_SCALE
        );
        assert_eq!(
            Settings::from_toml_lossy("ui_scale = 40.0").0.ui_scale,
            MAX_UI_SCALE
        );
        assert_eq!(Settings::from_toml_lossy("ui_scale = nan").0.ui_scale, 1.0);
        assert!(UI_SCALE_PRESETS.windows(2).all(|w| w[0] < w[1]));
        assert!(UI_SCALE_PRESETS.contains(&1.0));
    }

    #[test]
    fn extensions() {
        assert_eq!(PhotoFormat::Jpeg.extension(), "jpg");
        assert_eq!(PhotoFormat::Png.extension(), "png");
        assert_eq!(PhotoFormat::Bmp.extension(), "bmp");
    }
}

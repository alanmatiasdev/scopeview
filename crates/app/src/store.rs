//! Reading and writing the settings and calibration files in the user's config folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use directories::{ProjectDirs, UserDirs};
use scopeview_core::{CameraControlSet, ProfileSet, RecentFiles, Settings};

const SETTINGS_FILE: &str = "settings.toml";
const CALIBRATIONS_FILE: &str = "calibrations.toml";
const CONTROLS_FILE: &str = "camera-controls.toml";
const RECENT_FILE: &str = "recent.toml";

pub struct Store {
    dir: Option<PathBuf>,
}

/// Write through a temporary file and rename, so a crash never leaves half a file.
fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

fn read_if_present(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

impl Store {
    /// The platform config folder (`~/.config/scopeview` on Linux).
    pub fn new() -> Self {
        Self {
            dir: ProjectDirs::from("", "", "scopeview").map(|d| d.config_dir().to_path_buf()),
        }
    }

    #[cfg(test)]
    pub fn with_dir(dir: PathBuf) -> Self {
        Self { dir: Some(dir) }
    }

    pub fn load_settings(&self) -> (Settings, Option<String>) {
        match self
            .dir
            .as_ref()
            .and_then(|d| read_if_present(&d.join(SETTINGS_FILE)))
        {
            Some(text) => Settings::from_toml_lossy(&text),
            None => (Settings::default(), None),
        }
    }

    pub fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        match &self.dir {
            Some(dir) => write_atomic(&dir.join(SETTINGS_FILE), &settings.to_toml()),
            None => Ok(()),
        }
    }

    pub fn load_profiles(&self) -> (ProfileSet, Vec<String>) {
        match self
            .dir
            .as_ref()
            .and_then(|d| read_if_present(&d.join(CALIBRATIONS_FILE)))
        {
            Some(text) => ProfileSet::from_toml_lossy(&text),
            None => (ProfileSet::default(), Vec::new()),
        }
    }

    pub fn load_controls(&self) -> (CameraControlSet, Option<String>) {
        match self
            .dir
            .as_ref()
            .and_then(|d| read_if_present(&d.join(CONTROLS_FILE)))
        {
            Some(text) => CameraControlSet::from_toml_lossy(&text),
            None => (CameraControlSet::default(), None),
        }
    }

    pub fn save_controls(&self, controls: &CameraControlSet) -> io::Result<()> {
        match &self.dir {
            Some(dir) => write_atomic(&dir.join(CONTROLS_FILE), &controls.to_toml()),
            None => Ok(()),
        }
    }

    pub fn load_recent(&self) -> (RecentFiles, Option<String>) {
        match self
            .dir
            .as_ref()
            .and_then(|d| read_if_present(&d.join(RECENT_FILE)))
        {
            Some(text) => RecentFiles::from_toml_lossy(&text),
            None => (RecentFiles::default(), None),
        }
    }

    pub fn save_recent(&self, recent: &RecentFiles) -> io::Result<()> {
        match &self.dir {
            Some(dir) => write_atomic(&dir.join(RECENT_FILE), &recent.to_toml()),
            None => Ok(()),
        }
    }

    pub fn save_profiles(&self, profiles: &ProfileSet) -> io::Result<()> {
        match &self.dir {
            Some(dir) => write_atomic(&dir.join(CALIBRATIONS_FILE), &profiles.to_toml()),
            None => Ok(()),
        }
    }
}

/// Folder for photos when the settings leave it empty: `<Pictures>/ScopeView`.
pub fn default_photo_dir() -> PathBuf {
    UserDirs::new()
        .and_then(|d| {
            d.picture_dir()
                .map(Path::to_path_buf)
                .or_else(|| Some(d.home_dir().to_path_buf()))
        })
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ScopeView")
}

/// The folder photos go to for these settings.
pub fn photo_dir(settings: &Settings) -> PathBuf {
    if settings.save_dir.trim().is_empty() {
        default_photo_dir()
    } else {
        PathBuf::from(settings.save_dir.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scopeview_core::{PhotoFormat, Profile, Unit};

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("scopeview-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn settings_survive_a_restart() {
        let dir = temp_dir("settings");
        let settings = Settings {
            line_width: 5,
            photo_format: PhotoFormat::Png,
            last_device: Some("Cam".into()),
            ..Settings::default()
        };
        Store::with_dir(dir.clone())
            .save_settings(&settings)
            .unwrap();
        let (loaded, warning) = Store::with_dir(dir.clone()).load_settings();
        assert_eq!(warning, None);
        assert_eq!(loaded, settings);
        assert!(!dir.join("settings.toml.tmp").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn profiles_survive_a_restart() {
        let dir = temp_dir("profiles");
        let mut set = ProfileSet::default();
        set.add(Profile::new("10x", Unit::Mm, 400.0, 2.0, (1280, 720), 20260101).unwrap())
            .unwrap();
        Store::with_dir(dir.clone()).save_profiles(&set).unwrap();
        let (loaded, warnings) = Store::with_dir(dir.clone()).load_profiles();
        assert!(warnings.is_empty());
        assert_eq!(loaded, set);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn recent_files_survive_a_restart() {
        let dir = temp_dir("recent");
        let mut recent = RecentFiles::default();
        recent.push(PathBuf::from("/photos/a.jpg"));
        recent.push(PathBuf::from("/photos/b.avi"));
        Store::with_dir(dir.clone()).save_recent(&recent).unwrap();
        let (loaded, warning) = Store::with_dir(dir.clone()).load_recent();
        assert_eq!(warning, None);
        assert_eq!(loaded, recent);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_files_give_defaults() {
        let store = Store::with_dir(temp_dir("missing"));
        assert_eq!(store.load_settings(), (Settings::default(), None));
        assert!(store.load_profiles().0.profiles().is_empty());
        assert!(store.load_recent().0.is_empty());
    }

    #[test]
    fn a_custom_photo_folder_is_used() {
        let s = Settings {
            save_dir: " /data/photos ".into(),
            ..Settings::default()
        };
        assert_eq!(photo_dir(&s), PathBuf::from("/data/photos"));
        assert!(photo_dir(&Settings::default()).ends_with("ScopeView"));
    }
}

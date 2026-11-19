//! Camera control values saved per camera, so they can be restored on the next start.
//!
//! Keys are plain strings (camera name, control name) so this crate does not depend on the
//! camera crate; the app converts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedControl {
    pub value: i64,
    /// `Some(true)` when the control was left on its automatic mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto: Option<bool>,
}

/// Saved controls: camera name -> control name -> value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraControlSet {
    #[serde(default)]
    cameras: BTreeMap<String, BTreeMap<String, SavedControl>>,
}

impl CameraControlSet {
    pub fn get(&self, camera: &str) -> impl Iterator<Item = (&str, SavedControl)> {
        self.cameras
            .get(camera)
            .into_iter()
            .flat_map(|m| m.iter().map(|(k, v)| (k.as_str(), *v)))
    }

    /// Remember one control of one camera.
    pub fn set(&mut self, camera: &str, control: &str, saved: SavedControl) {
        self.cameras
            .entry(camera.to_string())
            .or_default()
            .insert(control.to_string(), saved);
    }

    /// Forget everything saved for a camera (used by "reset to defaults").
    pub fn clear_camera(&mut self, camera: &str) {
        self.cameras.remove(camera);
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("camera settings always serialize")
    }

    /// Parse a saved file; a broken one gives an empty set and a warning.
    pub fn from_toml_lossy(text: &str) -> (Self, Option<String>) {
        match toml::from_str::<Self>(text) {
            Ok(set) => (set, None),
            Err(error) => (
                Self::default(),
                Some(format!("camera controls file ignored: {error}")),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_keeps_per_camera_values() {
        let mut set = CameraControlSet::default();
        set.set(
            "Cam A",
            "brightness",
            SavedControl {
                value: 12,
                auto: None,
            },
        );
        set.set(
            "Cam A",
            "exposure",
            SavedControl {
                value: 300,
                auto: Some(false),
            },
        );
        set.set(
            "Cam B",
            "gain",
            SavedControl {
                value: -5,
                auto: None,
            },
        );
        let (loaded, warning) = CameraControlSet::from_toml_lossy(&set.to_toml());
        assert_eq!(warning, None);
        assert_eq!(loaded, set);
        let a: Vec<_> = loaded.get("Cam A").collect();
        assert_eq!(a.len(), 2);
        assert_eq!(loaded.get("Cam C").count(), 0);
    }

    #[test]
    fn camera_names_with_odd_characters_survive() {
        let mut set = CameraControlSet::default();
        set.set(
            "HD camera : \"x\" [1]",
            "hue",
            SavedControl {
                value: 1,
                auto: None,
            },
        );
        let (loaded, _) = CameraControlSet::from_toml_lossy(&set.to_toml());
        assert_eq!(loaded, set);
    }

    #[test]
    fn clear_forgets_one_camera() {
        let mut set = CameraControlSet::default();
        set.set(
            "A",
            "hue",
            SavedControl {
                value: 1,
                auto: None,
            },
        );
        set.set(
            "B",
            "hue",
            SavedControl {
                value: 2,
                auto: None,
            },
        );
        set.clear_camera("A");
        assert_eq!(set.get("A").count(), 0);
        assert_eq!(set.get("B").count(), 1);
    }

    #[test]
    fn broken_file_gives_empty_set_and_warning() {
        let (set, warning) = CameraControlSet::from_toml_lossy("cameras = [");
        assert_eq!(set, CameraControlSet::default());
        assert!(warning.is_some());
    }

    #[test]
    fn empty_file_is_fine() {
        assert_eq!(
            CameraControlSet::from_toml_lossy(""),
            (CameraControlSet::default(), None)
        );
    }
}

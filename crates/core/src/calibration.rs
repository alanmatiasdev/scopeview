//! Calibration profiles: a named scale tied to the resolution it was made at.

use serde::{Deserialize, Serialize};

use crate::{Scale, ScaleError, Unit};

/// Parse a length typed by the user. Accepts `.` or `,` as the decimal mark; only finite values
/// greater than zero are returned.
pub fn parse_length(text: &str) -> Option<f64> {
    let value: f64 = text.trim().replace(',', ".").parse().ok()?;
    (value.is_finite() && value > 0.0).then_some(value)
}

/// Most profiles kept at once.
pub const MAX_PROFILES: usize = 10;

/// One saved calibration. The scale is derived from the two lengths, never stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub unit: Unit,
    /// Length in pixels of the line drawn on the reference.
    pub pixel_length: f64,
    /// Real length the user entered for that line, in `unit`.
    pub entered_length: f64,
    /// Frame size when the calibration was made.
    pub width: u32,
    pub height: u32,
    /// Date as YYYYMMDD.
    pub date: u32,
}

impl Profile {
    pub fn new(
        name: &str,
        unit: Unit,
        pixel_length: f64,
        entered_length: f64,
        resolution: (u32, u32),
        date: u32,
    ) -> Result<Self, ScaleError> {
        Scale::from_calibration(pixel_length, entered_length, unit)?;
        let name = name.trim();
        Ok(Self {
            name: if name.is_empty() { "Calibration" } else { name }.to_string(),
            unit,
            pixel_length,
            entered_length,
            width: resolution.0,
            height: resolution.1,
            date,
        })
    }

    pub fn scale(&self) -> Result<Scale, ScaleError> {
        Scale::from_calibration(self.pixel_length, self.entered_length, self.unit)
    }

    /// False when the frame size differs from the one used to calibrate, so the scale is suspect.
    pub fn matches_resolution(&self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSetError {
    /// Already `MAX_PROFILES` profiles and no one has the same name to replace.
    Full,
}

impl std::fmt::Display for ProfileSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at most {MAX_PROFILES} calibrations can be saved")
    }
}

impl std::error::Error for ProfileSetError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameError {
    /// No profile at that index.
    Missing,
    /// Another profile already has that name.
    Taken,
}

impl std::fmt::Display for RenameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenameError::Missing => write!(f, "no such calibration"),
            RenameError::Taken => write!(f, "another calibration has that name"),
        }
    }
}

impl std::error::Error for RenameError {}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProfileSet {
    profiles: Vec<Profile>,
    active: Option<usize>,
}

#[derive(Serialize, Deserialize, Default)]
struct File {
    #[serde(default)]
    active: Option<usize>,
    #[serde(default)]
    profiles: Vec<Profile>,
}

impl ProfileSet {
    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }

    pub fn active_index(&self) -> Option<usize> {
        self.active
    }

    pub fn active(&self) -> Option<&Profile> {
        self.active.and_then(|i| self.profiles.get(i))
    }

    pub fn active_scale(&self) -> Option<Scale> {
        self.active().and_then(|p| p.scale().ok())
    }

    /// Go back to showing pixels.
    pub fn clear_active(&mut self) {
        self.active = None;
    }

    pub fn set_active(&mut self, index: usize) -> bool {
        if index < self.profiles.len() {
            self.active = Some(index);
            true
        } else {
            false
        }
    }

    /// Rename a profile. The name is trimmed and a blank one becomes `Calibration`.
    /// Names stay unique: a name used by another profile is rejected.
    pub fn rename(&mut self, index: usize, name: &str) -> Result<(), RenameError> {
        let name = name.trim();
        let name = if name.is_empty() { "Calibration" } else { name };
        if index >= self.profiles.len() {
            return Err(RenameError::Missing);
        }
        if self
            .profiles
            .iter()
            .enumerate()
            .any(|(i, p)| i != index && p.name == name)
        {
            return Err(RenameError::Taken);
        }
        self.profiles[index].name = name.to_string();
        Ok(())
    }

    /// Delete a profile. The active profile stays the same one (its index shifts); deleting the
    /// active profile goes back to showing pixels.
    pub fn remove(&mut self, index: usize) -> Option<Profile> {
        if index >= self.profiles.len() {
            return None;
        }
        let removed = self.profiles.remove(index);
        self.active = match self.active {
            Some(a) if a == index => None,
            Some(a) if a > index => Some(a - 1),
            other => other,
        };
        Some(removed)
    }

    /// Add a profile and make it active. A profile with the same name is replaced.
    pub fn add(&mut self, profile: Profile) -> Result<usize, ProfileSetError> {
        let index = match self.profiles.iter().position(|p| p.name == profile.name) {
            Some(i) => {
                self.profiles[i] = profile;
                i
            }
            None if self.profiles.len() >= MAX_PROFILES => return Err(ProfileSetError::Full),
            None => {
                self.profiles.push(profile);
                self.profiles.len() - 1
            }
        };
        self.active = Some(index);
        Ok(index)
    }

    pub fn to_toml(&self) -> String {
        let file = File {
            active: self.active,
            profiles: self.profiles.clone(),
        };
        toml::to_string_pretty(&file).expect("profiles always serialize")
    }

    /// Parse a saved file. A broken file gives an empty set; broken or surplus profiles are dropped.
    /// The second value lists what was ignored, for the UI to show.
    pub fn from_toml_lossy(text: &str) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let file: File = match toml::from_str(text) {
            Ok(file) => file,
            Err(error) => {
                warnings.push(format!("calibration file ignored: {error}"));
                return (Self::default(), warnings);
            }
        };
        let mut set = Self::default();
        let mut active = None;
        for (i, profile) in file.profiles.into_iter().enumerate() {
            if profile.scale().is_err() {
                warnings.push(format!(
                    "calibration '{}' is invalid and was dropped",
                    profile.name
                ));
                continue;
            }
            if set.profiles.len() >= MAX_PROFILES {
                warnings.push(format!(
                    "calibration '{}' dropped: limit reached",
                    profile.name
                ));
                continue;
            }
            if file.active == Some(i) {
                active = Some(set.profiles.len());
            }
            set.profiles.push(profile);
        }
        set.active = active;
        (set, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str) -> Profile {
        Profile::new(name, Unit::Mm, 200.0, 2.0, (1280, 720), 20260101).unwrap()
    }

    fn set_of(names: &[&str]) -> ProfileSet {
        let mut set = ProfileSet::default();
        for n in names {
            set.add(profile(n)).unwrap();
        }
        set
    }

    #[test]
    fn rename_trims_and_keeps_names_unique() {
        let mut set = set_of(&["a", "b"]);
        assert_eq!(set.rename(0, "  x  "), Ok(()));
        assert_eq!(set.profiles()[0].name, "x");
        assert_eq!(set.rename(0, "b"), Err(RenameError::Taken));
        assert_eq!(set.rename(0, "x"), Ok(()), "keeping the own name is fine");
        assert_eq!(set.rename(0, "   "), Ok(()));
        assert_eq!(set.profiles()[0].name, "Calibration");
        assert_eq!(set.rename(5, "z"), Err(RenameError::Missing));
    }

    #[test]
    fn remove_before_active_shifts_the_active_index() {
        let mut set = set_of(&["a", "b", "c"]);
        assert!(set.set_active(2));
        assert_eq!(set.remove(0).unwrap().name, "a");
        assert_eq!(set.active_index(), Some(1));
        assert_eq!(set.active().unwrap().name, "c");
    }

    #[test]
    fn remove_the_active_profile_clears_it() {
        let mut set = set_of(&["a", "b"]);
        assert!(set.set_active(1));
        set.remove(1);
        assert_eq!(set.active_index(), None);
        assert_eq!(set.profiles().len(), 1);
    }

    #[test]
    fn remove_after_active_keeps_it_and_missing_is_none() {
        let mut set = set_of(&["a", "b", "c"]);
        assert!(set.set_active(0));
        set.remove(2);
        assert_eq!(set.active_index(), Some(0));
        assert_eq!(set.remove(9), None);
    }

    #[test]
    fn removing_makes_room_at_the_cap() {
        let names: Vec<String> = (0..MAX_PROFILES).map(|i| format!("p{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut set = set_of(&refs);
        assert_eq!(set.add(profile("extra")), Err(ProfileSetError::Full));
        set.remove(3);
        assert!(set.add(profile("extra")).is_ok());
    }

    #[test]
    fn rejects_bad_lengths_and_pixel_unit() {
        assert!(Profile::new("a", Unit::Mm, 0.0, 1.0, (1, 1), 0).is_err());
        assert!(Profile::new("a", Unit::Mm, 1.0, -1.0, (1, 1), 0).is_err());
        assert!(Profile::new("a", Unit::Px, 1.0, 1.0, (1, 1), 0).is_err());
    }

    #[test]
    fn blank_name_gets_a_default() {
        let p = Profile::new("  ", Unit::Um, 10.0, 5.0, (1, 1), 0).unwrap();
        assert_eq!(p.name, "Calibration");
    }

    #[test]
    fn scale_is_derived() {
        assert_eq!(profile("a").scale().unwrap().pixels_per_unit(), 100.0);
    }

    #[test]
    fn resolution_mismatch_is_flagged() {
        let p = profile("a");
        assert!(p.matches_resolution(1280, 720));
        assert!(!p.matches_resolution(640, 480));
    }

    #[test]
    fn add_activates_and_replaces_same_name() {
        let mut set = ProfileSet::default();
        assert_eq!(set.add(profile("a")), Ok(0));
        assert_eq!(set.add(profile("b")), Ok(1));
        assert_eq!(set.active_index(), Some(1));
        assert_eq!(set.add(profile("a")), Ok(0));
        assert_eq!(set.profiles().len(), 2);
        assert_eq!(set.active_index(), Some(0));
    }

    #[test]
    fn caps_at_ten_profiles() {
        let mut set = ProfileSet::default();
        for i in 0..MAX_PROFILES {
            set.add(profile(&format!("p{i}"))).unwrap();
        }
        assert_eq!(set.add(profile("extra")), Err(ProfileSetError::Full));
        assert!(set.add(profile("p3")).is_ok());
    }

    #[test]
    fn toml_round_trip() {
        let mut set = ProfileSet::default();
        set.add(profile("a")).unwrap();
        set.add(profile("b")).unwrap();
        set.set_active(0);
        let (loaded, warnings) = ProfileSet::from_toml_lossy(&set.to_toml());
        assert!(warnings.is_empty());
        assert_eq!(loaded, set);
    }

    #[test]
    fn broken_file_loads_empty_with_warning() {
        let (set, warnings) = ProfileSet::from_toml_lossy("this is [not toml");
        assert!(set.profiles().is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn invalid_profiles_are_dropped_and_active_follows() {
        let text = r#"
active = 1
[[profiles]]
name = "bad"
unit = "mm"
pixel_length = 0.0
entered_length = 1.0
width = 1
height = 1
date = 0
[[profiles]]
name = "good"
unit = "um"
pixel_length = 10.0
entered_length = 5.0
width = 1
height = 1
date = 0
"#;
        let (set, warnings) = ProfileSet::from_toml_lossy(text);
        assert_eq!(warnings.len(), 1);
        assert_eq!(set.profiles().len(), 1);
        assert_eq!(set.active().unwrap().name, "good");
    }

    #[test]
    fn parses_typed_lengths() {
        assert_eq!(parse_length(" 2.5 "), Some(2.5));
        assert_eq!(parse_length("2,5"), Some(2.5));
        for bad in ["", "abc", "0", "-1", "nan", "inf", "1e999"] {
            assert_eq!(parse_length(bad), None, "{bad}");
        }
    }

    #[test]
    fn clear_active_goes_back_to_pixels() {
        let mut set = ProfileSet::default();
        set.add(profile("a")).unwrap();
        set.clear_active();
        assert!(set.active_scale().is_none());
        assert_eq!(set.profiles().len(), 1);
    }
}

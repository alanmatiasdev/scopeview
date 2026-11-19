//! Pure measurement logic: geometry, units, calibration profiles and shape values.
//!
//! No camera, no UI, no file access (callers read and write the TOML text).
//! Everything here is deterministic and unit-tested.

pub mod calibration;
pub mod camera_settings;
pub mod crosshair;
pub mod edit;
pub mod format;
pub mod geometry;
pub mod measure;
pub mod overlay;
pub mod recent;
pub mod scale;
pub mod settings;
pub mod units;

pub use calibration::{Profile, ProfileSet, RenameError};
pub use camera_settings::{CameraControlSet, SavedControl};
pub use geometry::{
    Angle, Circle, Concentric, ParallelLines, Point, RectShape, Segment, Stroke, TextNote,
};
pub use measure::{Measurement, MeasurementList};
pub use recent::{FileKind, RecentFiles};
pub use scale::{Scale, ScaleError};
pub use settings::{PhotoFormat, Settings, Theme};
pub use units::Unit;

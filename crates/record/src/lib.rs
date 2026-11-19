//! Video recording to AVI (MJPEG), with pause, a time limit and file splitting.
//!
//! Frames come from the app with the overlay primitives to burn in. A background thread paces
//! them to a constant frame rate, encodes JPEG and writes the AVI files, so the UI never waits.

mod avi;
mod naming;
mod pacing;
mod recorder;

pub use avi::AviWriter;
pub use naming::{unique_stamp, video_file_name};
pub use pacing::{format_elapsed, pace, Pace, MAX_FILL_SECONDS};
pub use recorder::{RecordConfig, RecordError, Recorder, Status, StopReason, SPLIT_BYTES};

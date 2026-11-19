//! Camera enumeration and live frame streaming.
//!
//! The public API hides the capture backend (`nokhwa`: V4L2, Media Foundation, AVFoundation),
//! so it can be swapped without touching the UI. See `docs/adr/0004-camera-backend.md`.

mod controls;
mod device;
mod error;
mod mailbox;
mod stream;

pub use controls::{snap, ControlCommand, ControlInfo, ControlKind, ControlValue};
pub use device::{
    init_permissions, list_devices, list_formats, DeviceId, DeviceInfo, FormatInfo, PixelKind,
    PixelPreference,
};
pub use error::CameraError;
pub use stream::{Frame, Poll, Stream, StreamState};

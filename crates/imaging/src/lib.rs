//! Draws measurement overlays onto camera frames and saves photos.
//!
//! Overlays come from `scopeview_core::overlay` as primitives in image pixel coordinates, so the
//! saved photo has the camera's resolution and matches what the app shows.

mod filter;
mod flip;
mod photo;
mod raster;
mod thumbnail;

pub use filter::{apply_mode, ImageMode, BINARY_THRESHOLD};
pub use flip::flip;
pub use photo::{save_photo, PhotoError, JPEG_QUALITY};
pub use raster::render_overlay;
pub use thumbnail::{load_thumbnail, thumbnail, thumbnail_path, write_thumbnail, THUMB_SIDE};

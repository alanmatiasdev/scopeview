//! The recent files panel: the latest saved photos and videos with thumbnails.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use scopeview_core::recent::FileKind;
use scopeview_i18n::tr;
use scopeview_imaging::{load_thumbnail, write_thumbnail};

use super::App;
use crate::open::open_path;

/// How long to wait before looking again for a thumbnail that was not there.
const RETRY: Duration = Duration::from_secs(2);

/// A thumbnail texture, or a note that there is none yet.
pub(super) struct Thumb {
    texture: Option<TextureHandle>,
    tried: Instant,
}

pub(super) type ThumbCache = HashMap<PathBuf, Thumb>;

impl App {
    /// Remember a file just saved: first in the list, saved to disk.
    pub(super) fn remember_file(&mut self, path: PathBuf) {
        self.thumbs.remove(&path);
        self.recent.push(path);
        if let Err(error) = self.store.save_recent(&self.recent) {
            self.notice = Some(tr!("save-error", error = error));
        }
    }

    /// Save a thumbnail of the first video frame, off the UI thread.
    pub(super) fn write_video_thumbnail(&self, frame: image::RgbaImage, video: &Path) {
        let video = video.to_path_buf();
        std::thread::spawn(move || {
            let _ = write_thumbnail(&frame, &video);
        });
    }

    fn thumb_texture(&mut self, ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
        if let Some(entry) = self.thumbs.get(path) {
            if entry.texture.is_some() || entry.tried.elapsed() < RETRY {
                return entry.texture.clone();
            }
        }
        let texture = load_thumbnail(path).map(|img| {
            let size = [img.width() as usize, img.height() as usize];
            let color = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
            ctx.load_texture(
                format!("thumb:{}", path.display()),
                color,
                TextureOptions::LINEAR,
            )
        });
        self.thumbs.insert(
            path.to_path_buf(),
            Thumb {
                texture: texture.clone(),
                tried: Instant::now(),
            },
        );
        texture
    }

    pub(super) fn recent_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(tr!("recent-title"));
        ui.separator();
        if self.recent.is_empty() {
            ui.label(tr!("recent-empty"));
            return;
        }
        let ctx = ui.ctx().clone();
        let mut open: Option<PathBuf> = None;
        let mut remove: Option<PathBuf> = None;
        for path in self.recent.files().to_vec() {
            ui.group(|ui| {
                match self.thumb_texture(&ctx, &path) {
                    Some(texture) => {
                        let response = ui.add(
                            egui::Image::new(&texture)
                                .max_width(128.0)
                                .sense(egui::Sense::click()),
                        );
                        if response.clicked() {
                            open = Some(path.clone());
                        }
                        response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    }
                    None => {
                        let label = if FileKind::of(&path) == FileKind::Video {
                            tr!("recent-video")
                        } else {
                            tr!("recent-no-preview")
                        };
                        if ui.button(label).clicked() {
                            open = Some(path.clone());
                        }
                    }
                }
                let name = path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                ui.label(name).on_hover_text(path.display().to_string());
                ui.horizontal(|ui| {
                    if ui.small_button(tr!("recent-open")).clicked() {
                        open = Some(path.clone());
                    }
                    if ui.small_button(tr!("recent-remove")).clicked() {
                        remove = Some(path.clone());
                    }
                });
            });
        }
        if let Some(path) = open {
            if let Err(error) = open_path(&path) {
                self.notice = Some(tr!(
                    "recent-open-error",
                    path = path.display(),
                    error = error
                ));
            }
        }
        if let Some(path) = remove {
            self.recent.remove(&path);
            self.thumbs.remove(&path);
            if let Err(error) = self.store.save_recent(&self.recent) {
                self.notice = Some(tr!("save-error", error = error));
            }
        }
    }
}

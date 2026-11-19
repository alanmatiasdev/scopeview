//! Recording from the live view: the buttons, feeding frames to the recorder thread and
//! reporting how it ended.

use std::time::Duration;

use chrono::Local;
use eframe::egui;
use scopeview_camera::StreamState;
use scopeview_core::settings::RECORD_LIMIT_PRESETS;
use scopeview_i18n::tr;
use scopeview_record::{format_elapsed, RecordConfig, Recorder, Status, StopReason};

use super::App;
use crate::store::photo_dir;

/// How often the toolbar timer refreshes while recording.
const TIMER_REFRESH: Duration = Duration::from_millis(100);

/// Text for a limit preset: `Unlimited`, `10 s`, `5 min`, `2 h`.
pub(super) fn limit_label(seconds: u32) -> String {
    match seconds {
        0 => tr!("limit-unlimited"),
        s if s < 60 => tr!("duration-seconds", n = s),
        s if s < 3600 => tr!("duration-minutes", n = s / 60),
        s => tr!("duration-hours", n = s / 3600),
    }
}

impl App {
    pub(super) fn is_recording(&self) -> bool {
        self.recorder.is_some()
    }

    pub(super) fn start_recording(&mut self) {
        if self.recorder.is_some() {
            return;
        }
        let Some(frame) = &self.last_frame else {
            self.notice = Some(tr!("record-no-frame"));
            return;
        };
        let fps = match &self.state {
            Some(StreamState::Running(format)) => format.fps,
            _ => 30,
        };
        let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
        let mut config = RecordConfig::new(
            photo_dir(&self.settings),
            &stamp,
            (frame.width, frame.height),
            fps.max(1),
        );
        let limit = self.settings.max_record_seconds;
        config.max_duration = (limit > 0).then(|| Duration::from_secs(u64::from(limit)));
        match Recorder::start(config) {
            Ok(recorder) => {
                let status = recorder.status();
                if let (Some(first), Some(image)) = (
                    status.files.first(),
                    image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone()),
                ) {
                    self.write_video_thumbnail(image, first);
                }
                self.record_status = Some(status);
                self.recorder = Some(recorder);
                self.notice = None;
            }
            Err(error) => self.notice = Some(tr!("record-error", error = error)),
        }
    }

    /// Stop and save. Does nothing when not recording.
    pub(super) fn stop_recording(&mut self) {
        if let Some(recorder) = self.recorder.take() {
            let status = recorder.stop();
            self.notice = Some(self.finished_notice(&status));
            if let Some(first) = status.files.first().cloned() {
                self.remember_file(first);
            }
        }
        self.record_status = None;
    }

    pub(super) fn toggle_pause(&mut self) {
        let Some(recorder) = &self.recorder else {
            return;
        };
        if self.record_status.as_ref().is_some_and(|s| s.paused) {
            recorder.resume();
        } else {
            recorder.pause();
        }
    }

    /// Hand the newest frame to the recorder, with the overlay to burn into it.
    pub(super) fn feed_recorder(&mut self) {
        if self.recorder.is_none() || self.record_status.as_ref().is_some_and(|s| s.paused) {
            return;
        }
        let Some(frame) = &self.last_frame else {
            return;
        };
        let Some(image) = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone())
        else {
            return;
        };
        let primitives = self.scene_primitives();
        if let Some(recorder) = &self.recorder {
            recorder.submit(image, primitives);
        }
    }

    /// Refresh the cached status and notice when the recording ended by itself (time limit,
    /// disk error).
    pub(super) fn poll_recorder(&mut self, ctx: &egui::Context) {
        let Some(recorder) = &self.recorder else {
            return;
        };
        let status = recorder.status();
        let finished = status.finished;
        self.record_status = Some(status);
        if finished {
            self.stop_recording();
        } else {
            ctx.request_repaint_after(TIMER_REFRESH);
        }
    }

    fn finished_notice(&self, status: &Status) -> String {
        if let Some(error) = &status.error {
            return tr!("record-error", error = error);
        }
        let Some(first) = status.files.first() else {
            return String::new();
        };
        match (status.reason, status.files.len()) {
            (Some(StopReason::TimeLimit), _) => tr!("record-limit", path = first.display()),
            (_, 1) => tr!("record-saved", path = first.display()),
            (_, n) => tr!("record-saved-parts", count = n, path = first.display()),
        }
    }

    /// `REC 1:05`, or `REC (paused) 1:05`, while recording.
    pub(super) fn recording_text(&self) -> Option<String> {
        let status = self.record_status.as_ref()?;
        self.recorder.as_ref()?;
        let time = format_elapsed(status.active);
        Some(if status.paused {
            tr!("record-paused", time = time)
        } else {
            tr!("record-running", time = time)
        })
    }

    pub(super) fn record_buttons(&mut self, ui: &mut egui::Ui) {
        if self.is_recording() {
            if ui.button(tr!("action-record-stop")).clicked() {
                self.stop_recording();
                return;
            }
            let paused = self.record_status.as_ref().is_some_and(|s| s.paused);
            let label = if paused {
                tr!("action-record-resume")
            } else {
                tr!("action-record-pause")
            };
            if ui.button(label).clicked() {
                self.toggle_pause();
            }
        } else if ui
            .add_enabled(
                self.last_frame.is_some(),
                egui::Button::new(tr!("action-record")),
            )
            .clicked()
        {
            self.start_recording();
        }
    }

    /// The recording time limit picker for the settings window. True when it changed.
    pub(super) fn record_limit_picker(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(tr!("settings-record-limit"));
            egui::ComboBox::from_id_salt("record-limit")
                .selected_text(limit_label(self.settings.max_record_seconds))
                .show_ui(ui, |ui| {
                    for seconds in RECORD_LIMIT_PRESETS {
                        changed |= ui
                            .selectable_value(
                                &mut self.settings.max_record_seconds,
                                seconds,
                                limit_label(seconds),
                            )
                            .changed();
                    }
                });
        });
        changed
    }
}

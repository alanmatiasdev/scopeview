//! The floating windows: calibration, settings and camera controls.

use chrono::{Datelike, Local};
use eframe::egui::{self, Color32};
use scopeview_camera::{ControlCommand, ControlKind};
use scopeview_core::calibration::{parse_length, MAX_PROFILES};
use scopeview_core::settings::UI_SCALE_PRESETS;
use scopeview_core::{Measurement, PhotoFormat, Profile, SavedControl, TextNote, Theme, Unit};
use scopeview_i18n::{tr, Language};

use super::App;
use crate::store::default_photo_dir;
use crate::theme;
use crate::tools::Tool;

impl App {
    pub(super) fn calibration_window(&mut self, ctx: &egui::Context) {
        let Some(draft) = &mut self.calibration else {
            return;
        };
        enum Outcome {
            Keep,
            Save,
            Cancel,
        }
        let mut outcome = Outcome::Keep;
        egui::Window::new(tr!("calibrate-title"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(tr!(
                    "calibrate-hint",
                    pixels = format!("{:.1}", draft.pixel_length)
                ));
                egui::Grid::new("calibrate-grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label(tr!("calibrate-name"));
                        ui.text_edit_singleline(&mut draft.name);
                        ui.end_row();
                        ui.label(tr!("calibrate-length"));
                        ui.horizontal(|ui| {
                            let edit = ui.add(
                                egui::TextEdit::singleline(&mut draft.length_text)
                                    .desired_width(80.0),
                            );
                            if !edit.has_focus() && draft.length_text.is_empty() {
                                edit.request_focus();
                            }
                            egui::ComboBox::from_id_salt("calibrate-unit")
                                .selected_text(draft.unit.suffix())
                                .width(60.0)
                                .show_ui(ui, |ui| {
                                    for unit in [Unit::Mm, Unit::Um] {
                                        ui.selectable_value(&mut draft.unit, unit, unit.suffix());
                                    }
                                });
                        });
                        ui.end_row();
                    });
                if let Some(error) = &draft.error {
                    ui.colored_label(Color32::LIGHT_RED, error);
                }
                ui.horizontal(|ui| {
                    if ui.button(tr!("calibrate-save")).clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        outcome = Outcome::Save;
                    }
                    if ui.button(tr!("calibrate-cancel")).clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        outcome = Outcome::Cancel;
                    }
                });
            });

        match outcome {
            Outcome::Keep => {}
            Outcome::Cancel => self.calibration = None,
            Outcome::Save => self.save_calibration(),
        }
    }

    pub(super) fn save_calibration(&mut self) {
        let Some(draft) = &mut self.calibration else {
            return;
        };
        let Some(length) = parse_length(&draft.length_text) else {
            draft.error = Some(tr!("calibrate-error-number"));
            return;
        };
        let today = Local::now().date_naive();
        let date = today.year() as u32 * 10_000 + today.month() * 100 + today.day();
        let profile = match Profile::new(
            &draft.name,
            draft.unit,
            draft.pixel_length,
            length,
            draft.resolution,
            date,
        ) {
            Ok(profile) => profile,
            Err(_) => {
                draft.error = Some(tr!("calibrate-error-number"));
                return;
            }
        };
        if self.profiles.add(profile).is_err() {
            draft.error = Some(tr!("calibrate-error-full", max = MAX_PROFILES));
            return;
        }
        self.calibration = None;
        self.builder.set_tool(Tool::None);
        self.save_profiles();
    }

    pub(super) fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        let mut changed = false;
        let was_prefer_mjpeg = self.settings.prefer_mjpeg;
        let was_look = (self.settings.theme, self.settings.high_contrast);
        let was_language = self.settings.language.clone();
        let was_scale = self.settings.ui_scale;
        egui::Window::new(tr!("settings-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(tr!("settings-folder"));
                changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut self.settings.save_dir)
                            .desired_width(320.0)
                            .hint_text(tr!(
                                "settings-folder-default",
                                path = default_photo_dir().display()
                            )),
                    )
                    .changed();
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(tr!("settings-line-color"));
                    changed |= ui
                        .color_edit_button_srgb(&mut self.settings.line_color)
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label(tr!("settings-line-width"));
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.settings.line_width,
                            1..=scopeview_core::settings::MAX_LINE_WIDTH,
                        ))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label(tr!("settings-text-color"));
                    changed |= ui
                        .color_edit_button_srgb(&mut self.settings.text_color)
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label(tr!("settings-text-size"));
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.settings.text_size,
                            scopeview_core::settings::MIN_TEXT_SIZE
                                ..=scopeview_core::settings::MAX_TEXT_SIZE,
                        ))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label(tr!("settings-grid-color"));
                    changed |= ui
                        .color_edit_button_srgb(&mut self.settings.grid_color)
                        .changed();
                });
                changed |= self.record_limit_picker(ui);
                ui.horizontal(|ui| {
                    ui.label(tr!("settings-photo-format"));
                    for (format, label) in [
                        (PhotoFormat::Jpeg, "JPEG"),
                        (PhotoFormat::Png, "PNG"),
                        (PhotoFormat::Bmp, "BMP"),
                    ] {
                        changed |= ui
                            .selectable_value(&mut self.settings.photo_format, format, label)
                            .changed();
                    }
                });
                changed |= ui
                    .checkbox(
                        &mut self.settings.prefer_mjpeg,
                        tr!("settings-prefer-mjpeg"),
                    )
                    .changed();
                changed |= ui
                    .checkbox(
                        &mut self.settings.smooth_scaling,
                        tr!("settings-smooth-scaling"),
                    )
                    .changed();
                ui.separator();
                changed |= self.language_picker(ui);
                ui.horizontal(|ui| {
                    ui.label(tr!("settings-theme"));
                    for (value, key) in [
                        (Theme::System, "theme-system"),
                        (Theme::Light, "theme-light"),
                        (Theme::Dark, "theme-dark"),
                    ] {
                        changed |= ui
                            .selectable_value(&mut self.settings.theme, value, tr!(key))
                            .changed();
                    }
                });
                changed |= ui
                    .checkbox(
                        &mut self.settings.high_contrast,
                        tr!("settings-high-contrast"),
                    )
                    .changed();
                ui.horizontal(|ui| {
                    ui.label(tr!("settings-ui-scale"));
                    egui::ComboBox::from_id_salt("ui-scale")
                        .selected_text(percent(self.settings.ui_scale))
                        .show_ui(ui, |ui| {
                            for scale in UI_SCALE_PRESETS {
                                changed |= ui
                                    .selectable_value(
                                        &mut self.settings.ui_scale,
                                        scale,
                                        percent(scale),
                                    )
                                    .changed();
                            }
                        });
                });
            });
        self.settings_open = open;
        if self.settings.language != was_language {
            scopeview_i18n::set_language(super::language_of(&self.settings));
        }
        if (self.settings.theme, self.settings.high_contrast) != was_look {
            theme::apply(ctx, self.settings.theme, self.settings.high_contrast);
        }
        if self.settings.ui_scale != was_scale {
            self.applied_scale = self.settings.ui_scale;
            ctx.set_zoom_factor(self.settings.ui_scale);
        }
        if changed {
            self.save_settings();
        }
        // The automatic format choice depends on the preference, so reopen the camera.
        if self.settings.prefer_mjpeg != was_prefer_mjpeg && self.format.is_none() {
            self.reset_view();
            self.start_stream(ctx);
        }
    }

    pub(super) fn profiles_window(&mut self, ctx: &egui::Context) {
        if !self.profile_manager.open {
            return;
        }
        let mut open = true;
        let mut activate: Option<usize> = None;
        let mut commit_rename = false;
        let mut delete: Option<usize> = None;
        let manager = &mut self.profile_manager;
        egui::Window::new(tr!("profiles-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if self.profiles.profiles().is_empty() {
                    ui.label(tr!("profiles-empty"));
                }
                for (i, profile) in self.profiles.profiles().iter().enumerate() {
                    ui.horizontal(|ui| {
                        let active = self.profiles.active_index() == Some(i);
                        if ui.radio(active, "").clicked() && !active {
                            activate = Some(i);
                        }
                        let renaming = matches!(&manager.rename, Some((r, _, _)) if *r == i);
                        if renaming {
                            if let Some((_, text, error)) = &mut manager.rename {
                                let edit =
                                    ui.add(egui::TextEdit::singleline(text).desired_width(140.0));
                                if edit.lost_focus()
                                    && ui.input(|input| input.key_pressed(egui::Key::Enter))
                                {
                                    commit_rename = true;
                                }
                                if ui.button(tr!("profiles-ok")).clicked() {
                                    commit_rename = true;
                                }
                                if ui.button(tr!("calibrate-cancel")).clicked() {
                                    manager.rename = None;
                                } else if let Some(error) = error {
                                    ui.colored_label(Color32::LIGHT_RED, error.as_str());
                                }
                            }
                        } else {
                            ui.label(&profile.name);
                            ui.weak(tr!(
                                "profiles-detail",
                                length = profile.entered_length,
                                unit = profile.unit.suffix(),
                                width = profile.width,
                                height = profile.height
                            ));
                            if manager.confirm_delete == Some(i) {
                                if ui.button(tr!("profiles-confirm-delete")).clicked() {
                                    delete = Some(i);
                                }
                                if ui.button(tr!("calibrate-cancel")).clicked() {
                                    manager.confirm_delete = None;
                                }
                            } else {
                                if ui.button(tr!("profiles-rename")).clicked() {
                                    manager.rename = Some((i, profile.name.clone(), None));
                                    manager.confirm_delete = None;
                                }
                                if ui.button(tr!("profiles-delete")).clicked() {
                                    manager.confirm_delete = Some(i);
                                    manager.rename = None;
                                }
                            }
                        }
                    });
                }
                ui.separator();
                ui.weak(tr!(
                    "profiles-count",
                    count = self.profiles.profiles().len(),
                    max = MAX_PROFILES
                ));
            });
        self.profile_manager.open = open;
        let mut changed = false;
        if let Some(i) = activate {
            changed |= self.profiles.set_active(i);
        }
        if commit_rename {
            if let Some((index, text, error)) = &mut self.profile_manager.rename {
                match self.profiles.rename(*index, text) {
                    Ok(()) => {
                        changed = true;
                        self.profile_manager.rename = None;
                    }
                    Err(_) => *error = Some(tr!("profiles-error-taken")),
                }
            }
        }
        if let Some(i) = delete {
            self.profiles.remove(i);
            self.profile_manager.confirm_delete = None;
            changed = true;
        }
        if changed {
            self.save_profiles();
        }
    }

    pub(super) fn text_window(&mut self, ctx: &egui::Context) {
        let Some(draft) = &mut self.text_draft else {
            return;
        };
        let mut open = true;
        let mut confirm = false;
        let mut cancel = false;
        egui::Window::new(tr!("text-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(tr!("text-prompt"));
                let edit = ui.add(egui::TextEdit::singleline(&mut draft.text).desired_width(260.0));
                if draft.focus {
                    edit.request_focus();
                    draft.focus = false;
                }
                if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    confirm = true;
                }
                ui.horizontal(|ui| {
                    if ui.button(tr!("text-place")).clicked() {
                        confirm = true;
                    }
                    if ui.button(tr!("calibrate-cancel")).clicked() {
                        cancel = true;
                    }
                });
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            cancel = true;
        }
        if confirm {
            let text = draft.text.trim();
            if !text.is_empty() {
                let note = TextNote::new(draft.pos, text);
                self.measurements.push(Measurement::Text(note));
            }
            self.text_draft = None;
        } else if cancel || !open {
            self.text_draft = None;
        }
    }

    pub(super) fn controls_window(&mut self, ctx: &egui::Context) {
        if !self.controls_open {
            return;
        }
        let mut open = true;
        let mut commands: Vec<ControlCommand> = Vec::new();
        let mut reset = false;
        egui::Window::new(tr!("controls-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if self.controls.is_empty() {
                    ui.label(tr!("controls-none"));
                    return;
                }
                egui::Grid::new("controls-grid")
                    .num_columns(3)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        for c in &mut self.controls {
                            ui.label(tr!(control_label_key(c.kind)));
                            if c.kind == ControlKind::AntiFlicker {
                                let mut chosen = c.value;
                                egui::ComboBox::from_id_salt("flicker")
                                    .selected_text(tr!(flicker_key(c.value)))
                                    .show_ui(ui, |ui| {
                                        for v in c.min..=c.max {
                                            ui.selectable_value(
                                                &mut chosen,
                                                v,
                                                tr!(flicker_key(v)),
                                            );
                                        }
                                    });
                                if chosen != c.value {
                                    c.value = chosen;
                                    commands.push(ControlCommand::Set(c.kind, chosen));
                                }
                            } else {
                                let manual = c.active && c.auto != Some(true);
                                let response = ui.add_enabled(
                                    manual,
                                    egui::Slider::new(&mut c.value, c.min..=c.max)
                                        .step_by(c.step as f64),
                                );
                                if response.changed() {
                                    commands.push(ControlCommand::Set(c.kind, c.value));
                                }
                            }
                            match c.auto {
                                Some(auto) => {
                                    let mut on = auto;
                                    if ui.checkbox(&mut on, tr!("controls-auto")).changed() {
                                        c.auto = Some(on);
                                        commands.push(ControlCommand::SetAuto(c.kind, on));
                                    }
                                }
                                None => {
                                    ui.label("");
                                }
                            }
                            ui.end_row();
                        }
                    });
                ui.add_space(4.0);
                if ui.button(tr!("controls-reset")).clicked() {
                    reset = true;
                }
            });
        self.controls_open = open;

        let Some(name) = self.current_device().map(|d| d.name.clone()) else {
            return;
        };
        if reset {
            if let Some(stream) = &self.stream {
                stream.control(ControlCommand::Reset);
            }
            self.saved_controls.clear_camera(&name);
            self.save_controls();
            return;
        }
        if commands.is_empty() {
            return;
        }
        let mut save = false;
        for command in commands {
            if let Some(stream) = &self.stream {
                stream.control(command);
            }
            // Written on release rather than on every slider step, to avoid a file write per pixel.
            save = true;
            let kind = match command {
                ControlCommand::Set(k, _) | ControlCommand::SetAuto(k, _) => k,
                ControlCommand::Reset => continue,
            };
            if let Some(c) = self.controls.iter().find(|c| c.kind == kind) {
                self.saved_controls.set(
                    &name,
                    kind.key(),
                    SavedControl {
                        value: c.value,
                        auto: c.auto,
                    },
                );
            }
        }
        if save && !ctx.input(|i| i.pointer.any_down()) {
            self.save_controls();
        } else if save {
            self.controls_dirty = true;
        }
    }
}

fn control_label_key(kind: ControlKind) -> &'static str {
    match kind {
        ControlKind::Brightness => "control-brightness",
        ControlKind::Contrast => "control-contrast",
        ControlKind::Saturation => "control-saturation",
        ControlKind::Hue => "control-hue",
        ControlKind::Sharpness => "control-sharpness",
        ControlKind::Gamma => "control-gamma",
        ControlKind::Gain => "control-gain",
        ControlKind::WhiteBalance => "control-white-balance",
        ControlKind::Exposure => "control-exposure",
        ControlKind::AntiFlicker => "control-anti-flicker",
        ControlKind::BacklightCompensation => "control-backlight",
    }
}

fn flicker_key(value: i64) -> &'static str {
    match value {
        0 => "control-flicker-off",
        1 => "control-flicker-50",
        2 => "control-flicker-60",
        _ => "control-flicker-auto",
    }
}

fn percent(scale: f32) -> String {
    format!("{}%", (scale * 100.0).round())
}

impl App {
    /// The language drop-down: "System default" plus each language under its own name.
    fn language_picker(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let selected = match self
            .settings
            .language
            .as_deref()
            .and_then(Language::from_code)
        {
            Some(language) => language.autonym().to_string(),
            None => tr!("language-system"),
        };
        ui.horizontal(|ui| {
            ui.label(tr!("settings-language"));
            egui::ComboBox::from_id_salt("language")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.settings.language.is_none(), tr!("language-system"))
                        .clicked()
                        && self.settings.language.is_some()
                    {
                        self.settings.language = None;
                        changed = true;
                    }
                    for language in Language::ALL {
                        let chosen = self.settings.language.as_deref() == Some(language.code());
                        if ui.selectable_label(chosen, language.autonym()).clicked() && !chosen {
                            self.settings.language = Some(language.code().to_string());
                            changed = true;
                        }
                    }
                });
        });
        changed
    }
}

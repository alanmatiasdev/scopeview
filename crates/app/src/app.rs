use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use chrono::Local;
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use scopeview_camera::{
    init_permissions, list_devices, list_formats, ControlInfo, ControlKind, ControlValue,
    DeviceInfo, FormatInfo, Frame, PixelPreference, Stream, StreamState,
};
use scopeview_core::crosshair;
use scopeview_core::overlay::{self, Primitive, Style};
use scopeview_core::{
    CameraControlSet, MeasurementList, ProfileSet, RecentFiles, Scale, Settings, Unit,
};
use scopeview_i18n::{tr, Language};
use scopeview_imaging::ImageMode;
use scopeview_record::{Recorder, Status as RecordStatus};

use crate::paint::paint;
use crate::selection::{device_list_changed, pick_device};
use crate::shortcuts::Action;
use crate::store::{photo_dir, Store};
use crate::theme;
use crate::tools::{Builder, Outcome, Selector, Tool};
use crate::view::ViewTransform;

mod help;
mod recent;
mod recording;
mod windows;

/// How often to look for added or removed cameras while no video is flowing.
const DEVICE_REFRESH: Duration = Duration::from_secs(2);

/// How close, in screen points, the pointer must be to a shape or handle to grab it.
const SELECT_TOLERANCE: f64 = 6.0;

/// State of the calibration manager window: which row is being renamed or deleted.
#[derive(Default)]
struct ProfileManager {
    open: bool,
    /// Row being renamed, its text so far and the last error.
    rename: Option<(usize, String, Option<String>)>,
    /// Row waiting for the delete confirmation.
    confirm_delete: Option<usize>,
}

/// Text being typed for a note that is about to be placed.
struct TextDraft {
    pos: scopeview_core::Point,
    text: String,
    /// The field takes focus the first time the window shows.
    focus: bool,
}

struct CalibrationDraft {
    pixel_length: f64,
    resolution: (u32, u32),
    name: String,
    unit: Unit,
    length_text: String,
    error: Option<String>,
}

pub struct App {
    devices: Vec<DeviceInfo>,
    device: Option<usize>,
    formats: Vec<FormatInfo>,
    /// `None` means automatic format.
    format: Option<usize>,
    stream: Option<Stream>,
    state: Option<StreamState>,
    texture: Option<TextureHandle>,
    last_frame: Option<Frame>,
    error: Option<String>,
    last_refresh: Instant,

    store: Store,
    settings: Settings,
    profiles: ProfileSet,
    builder: Builder,
    selector: Selector,
    measurements: MeasurementList,
    calibration: Option<CalibrationDraft>,
    profile_manager: ProfileManager,
    text_draft: Option<TextDraft>,
    recorder: Option<Recorder>,
    recent: RecentFiles,
    recent_open: bool,
    thumbs: recent::ThumbCache,
    /// Recorder status as of the last poll, for the toolbar and status bar.
    record_status: Option<RecordStatus>,
    settings_open: bool,
    controls_open: bool,
    help_open: bool,
    about_open: bool,
    /// The interface zoom this app last set. A different value from egui means the user pressed
    /// the zoom keys, and the new value is saved.
    applied_scale: f32,
    flip_h: bool,
    flip_v: bool,
    image_mode: ImageMode,
    crosshair: bool,
    /// Controls the open camera reports, as of the last poll.
    controls: Vec<ControlInfo>,
    saved_controls: CameraControlSet,
    /// Control values changed while the mouse was held; written once it is released.
    controls_dirty: bool,
    notice: Option<String>,
    photo_result: Option<Receiver<Result<PathBuf, String>>>,
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        let store = Store::new();
        let (settings, settings_warning) = store.load_settings();
        scopeview_i18n::set_language(language_of(&settings));
        theme::apply(ctx, settings.theme, settings.high_contrast);
        let ui_scale = settings.ui_scale;
        ctx.set_zoom_factor(ui_scale);
        let (profiles, mut warnings) = store.load_profiles();
        let (saved_controls, controls_warning) = store.load_controls();
        let (mut recent, recent_warning) = store.load_recent();
        // Files deleted since the last run drop out of the list.
        recent.retain(|path| path.exists());
        warnings.extend(recent_warning);
        warnings.extend(settings_warning);
        warnings.extend(controls_warning);
        let mut app = Self {
            devices: Vec::new(),
            device: None,
            formats: Vec::new(),
            format: None,
            stream: None,
            state: None,
            texture: None,
            last_frame: None,
            error: None,
            last_refresh: Instant::now(),
            store,
            settings,
            profiles,
            builder: Builder::default(),
            selector: Selector::default(),
            measurements: MeasurementList::default(),
            calibration: None,
            profile_manager: ProfileManager::default(),
            text_draft: None,
            recorder: None,
            recent,
            recent_open: false,
            thumbs: recent::ThumbCache::new(),
            record_status: None,
            settings_open: false,
            controls_open: false,
            help_open: false,
            about_open: false,
            applied_scale: ui_scale,
            flip_h: false,
            flip_v: false,
            image_mode: ImageMode::default(),
            crosshair: false,
            controls: Vec::new(),
            saved_controls,
            controls_dirty: false,
            notice: (!warnings.is_empty()).then(|| warnings.join(" ")),
            photo_result: None,
        };
        if let Err(error) = init_permissions() {
            app.error = Some(error.to_string());
        }
        app.refresh_devices(ctx);
        app
    }

    fn current_device(&self) -> Option<&DeviceInfo> {
        self.device.and_then(|i| self.devices.get(i))
    }

    fn choose_tool(&mut self, tool: Tool) {
        self.builder.set_tool(tool);
        self.selector.reset();
    }

    fn delete_last(&mut self) {
        self.measurements.delete_last();
        self.selector.validate(&self.measurements);
    }

    fn delete_all(&mut self) {
        self.measurements.clear();
        self.selector.reset();
    }

    /// Toggle the horizontal or vertical flip. Measurements are in image pixels, which just
    /// moved, so they are dropped.
    fn flip(&mut self, horizontal: bool) {
        if horizontal {
            self.flip_h = !self.flip_h;
        } else {
            self.flip_v = !self.flip_v;
        }
        self.clear_work();
    }

    /// Follow the interface zoom: the user may have changed it with the zoom keys.
    fn sync_zoom(&mut self, ctx: &egui::Context) {
        use scopeview_core::settings::{MAX_UI_SCALE, MIN_UI_SCALE};
        let zoom = ctx.zoom_factor();
        if (zoom - self.applied_scale).abs() < 1e-3 {
            return;
        }
        let clamped = zoom.clamp(MIN_UI_SCALE, MAX_UI_SCALE);
        if (clamped - zoom).abs() > 1e-3 {
            ctx.set_zoom_factor(clamped);
        }
        self.applied_scale = clamped;
        self.settings.ui_scale = clamped;
        self.save_settings();
    }

    fn running(&self) -> bool {
        matches!(self.state, Some(StreamState::Running(_)))
    }

    fn frame_size(&self) -> Option<(u32, u32)> {
        self.last_frame.as_ref().map(|f| (f.width, f.height))
    }

    fn scale(&self) -> Option<Scale> {
        self.profiles.active_scale()
    }

    fn style(&self) -> Style {
        let [r, g, b] = self.settings.line_color;
        let [tr, tg, tb] = self.settings.text_color;
        let [gr, gg, gb] = self.settings.grid_color;
        Style {
            line_color: [r, g, b, 255],
            line_width: self.settings.line_width as f32,
            text_color: [tr, tg, tb, 255],
            text_size: self.settings.text_size as f32,
            grid_color: [gr, gg, gb, 255],
        }
    }

    // ---- persistence ----

    fn save_settings(&mut self) {
        if let Err(error) = self.store.save_settings(&self.settings) {
            self.notice = Some(tr!("save-error", error = error));
        }
    }

    fn save_controls(&mut self) {
        self.controls_dirty = false;
        if let Err(error) = self.store.save_controls(&self.saved_controls) {
            self.notice = Some(tr!("save-error", error = error));
        }
    }

    /// Saved control values for a camera, ready to apply when its stream opens.
    fn saved_for(&self, name: &str) -> Vec<ControlValue> {
        self.saved_controls
            .get(name)
            .filter_map(|(key, saved)| {
                ControlKind::from_key(key).map(|kind| ControlValue {
                    kind,
                    value: saved.value,
                    auto: saved.auto,
                })
            })
            .collect()
    }

    fn save_profiles(&mut self) {
        if let Err(error) = self.store.save_profiles(&self.profiles) {
            self.notice = Some(tr!("save-error", error = error));
        }
    }

    // ---- camera ----

    fn refresh_devices(&mut self, ctx: &egui::Context) {
        self.last_refresh = Instant::now();
        let found = match list_devices() {
            Ok(found) => found,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        // On the first look there is no current device yet: prefer the one used last time.
        let previous = self
            .current_device()
            .map(|d| d.name.clone())
            .or_else(|| self.settings.last_device.clone());
        let previous_present = found.iter().any(|d| Some(&d.name) == previous.as_ref());

        // While the selected camera is unplugged the capture thread keeps retrying on its own;
        // do not jump to another camera or drop the stream behind the user's back.
        let waiting = self.stream.is_some()
            && matches!(self.state, Some(StreamState::Reconnecting(_)))
            && !previous_present;
        if waiting {
            return;
        }
        if !device_list_changed(&self.devices, &found) && self.stream.is_some() {
            return;
        }

        self.devices = found;
        self.device = pick_device(&self.devices, previous.as_deref());
        let now = self.current_device().map(|d| d.name.clone());
        if now != previous || self.stream.is_none() {
            self.select_device(ctx);
        }
    }

    /// Forget everything tied to the current picture: the frame, and measurements in its pixels.
    fn reset_view(&mut self) {
        self.texture = None;
        self.last_frame = None;
        self.clear_work();
    }

    /// Drop measurements and anything half drawn: they refer to pixels that no longer apply.
    fn clear_work(&mut self) {
        self.measurements.clear();
        self.selector.reset();
        self.builder.cancel();
        self.calibration = None;
    }

    fn select_device(&mut self, ctx: &egui::Context) {
        self.stop_recording();
        // Dropping the stream joins its thread, which releases the device before we reopen it.
        self.stream = None;
        self.state = None;
        self.reset_view();
        self.formats.clear();
        self.format = None;
        self.controls.clear();
        self.error = None;
        let Some(device) = self.current_device().cloned() else {
            return;
        };
        if self.settings.last_device.as_deref() != Some(device.name.as_str()) {
            self.settings.last_device = Some(device.name.clone());
            self.save_settings();
        }
        match list_formats(&device) {
            Ok(formats) => self.formats = formats,
            Err(error) => self.error = Some(tr!("error-list-formats", error = error)),
        }
        self.start_stream(ctx);
    }

    fn start_stream(&mut self, ctx: &egui::Context) {
        self.stop_recording();
        self.stream = None;
        let Some(device) = self.current_device().cloned() else {
            return;
        };
        let format = self.format.and_then(|i| self.formats.get(i)).copied();
        self.state = Some(StreamState::Starting);
        self.controls.clear();
        let saved = self.saved_for(&device.name);
        let ctx = ctx.clone();
        let preference = if self.settings.prefer_mjpeg {
            PixelPreference::Mjpeg
        } else {
            PixelPreference::Raw
        };
        self.stream = Some(Stream::start(
            device,
            format,
            saved,
            preference,
            move || ctx.request_repaint(),
        ));
    }

    fn poll_stream(&mut self, ctx: &egui::Context) {
        let Some(stream) = &self.stream else {
            return;
        };
        let poll = stream.poll();
        self.state = Some(poll.state);
        if let Some(controls) = poll.controls {
            self.controls = controls;
        }
        if let Some(mut frame) = poll.frame {
            if self.flip_h || self.flip_v || self.image_mode != ImageMode::Normal {
                transform_frame(&mut frame, self.flip_h, self.flip_v, self.image_mode);
            }
            // Measurements are in image pixels; a different frame size makes them meaningless.
            if self
                .frame_size()
                .is_some_and(|s| s != (frame.width, frame.height))
            {
                self.clear_work();
            }
            let size = [frame.width as usize, frame.height as usize];
            let image = ColorImage::from_rgba_unmultiplied(size, &frame.rgba);
            let options = if self.settings.smooth_scaling {
                TextureOptions::LINEAR
            } else {
                TextureOptions::NEAREST
            };
            match &mut self.texture {
                Some(texture) => texture.set(image, options),
                None => self.texture = Some(ctx.load_texture("camera", image, options)),
            }
            self.last_frame = Some(frame);
            self.feed_recorder();
        }
    }

    // ---- measuring ----

    fn handle_outcome(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Nothing => {}
            Outcome::Done(measurement) => self.measurements.push(measurement),
            Outcome::PlaceText(pos) => {
                self.text_draft = Some(TextDraft {
                    pos,
                    text: String::new(),
                    focus: true,
                });
            }
            Outcome::Calibrate(segment) => match self.frame_size() {
                Some(resolution) => {
                    self.calibration = Some(CalibrationDraft {
                        pixel_length: segment.length(),
                        resolution,
                        name: tr!(
                            "calibration-default-name",
                            number = self.profiles.profiles().len() + 1
                        ),
                        unit: Unit::Mm,
                        length_text: String::new(),
                        error: None,
                    });
                }
                None => self.notice = Some(tr!("calibrate-error-no-frame")),
            },
        }
    }

    /// What is burned into photos and drawn on screen: the crosshair and the measurements.
    fn scene_primitives(&self) -> Vec<Primitive> {
        let scale = self.scale();
        let style = self.style();
        let mut primitives = match (self.crosshair, self.frame_size()) {
            (true, Some(size)) => crosshair::build(size, scale.as_ref(), &style),
            _ => Vec::new(),
        };
        primitives.extend(overlay::build(
            self.measurements.iter(),
            scale.as_ref(),
            &style,
        ));
        primitives
    }

    /// The primitives for the screen: the scene, selection handles and the shape being drawn.
    fn overlay_primitives(&self) -> Vec<Primitive> {
        let scale = self.scale();
        let style = self.style();
        let mut primitives = self.scene_primitives();
        if let Some(m) = self
            .selector
            .selected()
            .and_then(|i| self.measurements.get(i))
        {
            primitives.extend(overlay::handles(m, &style));
        }
        if let Some(preview) = self.builder.preview() {
            let scale = if preview.use_scale {
                scale.as_ref()
            } else {
                None
            };
            primitives.extend(overlay::build_one(&preview.measurement, scale, &style));
        }
        primitives
    }

    // ---- photo ----

    fn take_photo(&mut self) {
        if self.photo_result.is_some() {
            return;
        }
        let Some(frame) = &self.last_frame else {
            self.notice = Some(tr!("photo-no-frame"));
            return;
        };
        let Some(image) = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone())
        else {
            self.notice = Some(tr!("photo-error", error = tr!("error-invalid-frame")));
            return;
        };
        let primitives = self.scene_primitives();
        let dir = photo_dir(&self.settings);
        let format = self.settings.photo_format;
        let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
        let (tx, rx) = mpsc::channel();
        self.photo_result = Some(rx);
        std::thread::spawn(move || {
            let result = scopeview_imaging::save_photo(&image, &primitives, &dir, format, &stamp)
                .map_err(|e| e.to_string());
            let _ = tx.send(result);
        });
    }

    fn poll_photo(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.photo_result else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                self.notice = Some(match result {
                    Ok(path) => {
                        let text = tr!("photo-saved", path = path.display());
                        self.remember_file(path);
                        text
                    }
                    Err(error) => tr!("photo-error", error = error),
                });
                self.photo_result = None;
            }
            Err(TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
            Err(TryRecvError::Disconnected) => {
                self.notice = Some(tr!("photo-error", error = tr!("error-worker-stopped")));
                self.photo_result = None;
            }
        }
    }

    // ---- UI ----

    fn status_text(&self) -> String {
        if let Some(error) = &self.error {
            return error.clone();
        }
        match (&self.state, self.frame_size()) {
            (None, _) => tr!("status-no-camera"),
            (Some(StreamState::Starting), _) => tr!("status-opening"),
            (Some(StreamState::Running(format)), Some((w, h))) => {
                tr!("status-live", width = w, height = h, format = format)
            }
            (Some(StreamState::Running(format)), None) => tr!("status-waiting", format = format),
            (Some(StreamState::Reconnecting(reason)), _) => {
                tr!("status-reconnecting", reason = reason)
            }
        }
    }

    /// A warning when the active calibration was made at another resolution than the live image.
    fn mismatch_warning(&self) -> Option<String> {
        let (w, h) = self.frame_size()?;
        let profile = self.profiles.active()?;
        (!profile.matches_resolution(w, h)).then(|| {
            tr!(
                "calibration-mismatch",
                width = profile.width,
                height = profile.height
            )
        })
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) -> Toolbar {
        let mut action = Toolbar::default();
        ui.horizontal(|ui| {
            ui.label(tr!("toolbar-camera"));
            let selected = self
                .current_device()
                .map_or(tr!("device-none"), |d| d.name.clone());
            egui::ComboBox::from_id_salt("device")
                .selected_text(selected)
                .width(260.0)
                .show_ui(ui, |ui| {
                    for (i, device) in self.devices.iter().enumerate() {
                        let chosen = self.device == Some(i);
                        if ui.selectable_label(chosen, &device.name).clicked() && !chosen {
                            action.device = Some(i);
                        }
                    }
                });

            ui.label(tr!("toolbar-format"));
            let selected = self
                .format
                .and_then(|i| self.formats.get(i))
                .map_or(tr!("format-automatic"), ToString::to_string);
            ui.add_enabled_ui(!self.formats.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("format")
                    .selected_text(selected)
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(self.format.is_none(), tr!("format-automatic"))
                            .clicked()
                            && self.format.is_some()
                        {
                            action.format = Some(None);
                        }
                        for (i, format) in self.formats.iter().enumerate() {
                            let chosen = self.format == Some(i);
                            if ui.selectable_label(chosen, format.to_string()).clicked() && !chosen
                            {
                                action.format = Some(Some(i));
                            }
                        }
                    });
            });

            if ui.button(tr!("toolbar-refresh")).clicked() {
                action.refresh = true;
            }
        });
        action
    }

    fn tools_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(tr!("tool-label"));
            for (tool, key) in [
                (Tool::None, "tool-none"),
                (Tool::Select, "tool-select"),
                (Tool::Line, "tool-line"),
                (Tool::Rect, "tool-rect"),
                (Tool::Circle, "tool-circle"),
                (Tool::Concentric, "tool-concentric"),
                (Tool::Parallel, "tool-parallel"),
                (Tool::Angle, "tool-angle"),
                (Tool::Text, "tool-text"),
                (Tool::Pencil, "tool-pencil"),
                (Tool::Calibrate, "tool-calibrate"),
            ] {
                let response = ui.selectable_label(self.builder.tool() == tool, tr!(key));
                if Self::with_shortcut(ui.ctx(), response, Action::Tool(tool)).clicked() {
                    self.choose_tool(tool);
                }
            }
            ui.separator();

            ui.label(tr!("calibration-label"));
            let selected = self
                .profiles
                .active()
                .map_or(tr!("calibration-none"), |p| p.name.clone());
            let mut chosen: Option<Option<usize>> = None;
            egui::ComboBox::from_id_salt("profile")
                .selected_text(selected)
                .width(160.0)
                .show_ui(ui, |ui| {
                    let none = self.profiles.active_index().is_none();
                    if ui.selectable_label(none, tr!("calibration-none")).clicked() && !none {
                        chosen = Some(None);
                    }
                    for (i, profile) in self.profiles.profiles().iter().enumerate() {
                        let active = self.profiles.active_index() == Some(i);
                        if ui.selectable_label(active, &profile.name).clicked() && !active {
                            chosen = Some(Some(i));
                        }
                    }
                });
            match chosen {
                Some(Some(i)) => {
                    self.profiles.set_active(i);
                    self.save_profiles();
                }
                Some(None) => {
                    self.profiles.clear_active();
                    self.save_profiles();
                }
                None => {}
            }
            if ui.button(tr!("calibration-manage")).clicked() {
                self.profile_manager.open = !self.profile_manager.open;
                self.profile_manager.rename = None;
                self.profile_manager.confirm_delete = None;
            }
            ui.separator();

            let ctx = ui.ctx().clone();
            let has_any = !self.measurements.is_empty();
            let response = ui.add_enabled(has_any, egui::Button::new(tr!("action-delete-last")));
            if Self::with_shortcut(&ctx, response, Action::DeleteLast).clicked() {
                self.delete_last();
            }
            let response = ui.add_enabled(has_any, egui::Button::new(tr!("action-delete-all")));
            if Self::with_shortcut(&ctx, response, Action::DeleteAll).clicked() {
                self.delete_all();
            }
            ui.separator();

            let response = ui.selectable_label(self.flip_h, tr!("action-flip-h"));
            if Self::with_shortcut(&ctx, response, Action::FlipHorizontal).clicked() {
                self.flip(true);
            }
            let response = ui.selectable_label(self.flip_v, tr!("action-flip-v"));
            if Self::with_shortcut(&ctx, response, Action::FlipVertical).clicked() {
                self.flip(false);
            }
            let response = ui.checkbox(&mut self.crosshair, tr!("action-crosshair"));
            Self::with_shortcut(&ctx, response, Action::Crosshair);
            ui.label(tr!("mode-label"));
            egui::ComboBox::from_id_salt("image-mode")
                .selected_text(mode_label(self.image_mode))
                .show_ui(ui, |ui| {
                    for mode in [
                        ImageMode::Normal,
                        ImageMode::Gray,
                        ImageMode::Binary,
                        ImageMode::Invert,
                        ImageMode::Emboss,
                    ] {
                        ui.selectable_value(&mut self.image_mode, mode, mode_label(mode));
                    }
                });
            ui.separator();

            let can_photo = self.last_frame.is_some() && self.photo_result.is_none();
            let response = ui.add_enabled(can_photo, egui::Button::new(tr!("action-photo")));
            if Self::with_shortcut(&ctx, response, Action::Photo).clicked() {
                self.take_photo();
            }
            self.record_buttons(ui);
            ui.separator();
            if ui
                .selectable_label(self.recent_open, tr!("action-recent"))
                .clicked()
            {
                self.recent_open = !self.recent_open;
            }
            if ui.button(tr!("action-controls")).clicked() {
                self.controls_open = !self.controls_open;
            }
            let response = ui.button(tr!("action-settings"));
            if Self::with_shortcut(&ctx, response, Action::Settings).clicked() {
                self.settings_open = !self.settings_open;
            }
            let response = ui.button(tr!("action-help"));
            if Self::with_shortcut(&ctx, response, Action::Help).clicked() {
                self.help_open = !self.help_open;
            }
            if ui.button(tr!("action-about")).clicked() {
                self.about_open = !self.about_open;
            }
        });
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let (Some(texture), Some(size)) = (&self.texture, self.frame_size()) else {
            return;
        };
        let Some(view) = ViewTransform::fit(response.rect, size) else {
            return;
        };
        painter.image(
            texture.id(),
            view.image_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        let tool = self.builder.tool();
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.builder.cancel();
            self.selector.reset();
        }
        if tool == Tool::Select && ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            self.selector.delete_selected(&mut self.measurements);
        }

        let origin = ui.input(|i| i.pointer.press_origin());
        let at = |pos| view.to_image_clamped(pos);
        if tool == Tool::Select {
            let tolerance = SELECT_TOLERANCE / f64::from(view.scale);
            if response.drag_started() {
                if let Some(pos) = origin.or_else(|| response.interact_pointer_pos()) {
                    self.selector.press(
                        &self.measurements,
                        view.to_image(pos),
                        tolerance,
                        f64::from(self.settings.text_size),
                    );
                }
            } else if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.selector.press(
                        &self.measurements,
                        view.to_image(pos),
                        tolerance,
                        f64::from(self.settings.text_size),
                    );
                    self.selector.release();
                }
            }
            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.selector
                        .drag(&mut self.measurements, view.to_image(pos), at(pos));
                }
            }
            if response.drag_stopped() {
                self.selector.release();
            }
            if self.selector.is_dragging() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            } else if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        } else if tool != Tool::None {
            if let Some(pos) = response
                .hover_pos()
                .or_else(|| response.interact_pointer_pos())
            {
                self.builder.pointer_move(at(pos));
            }
            // While the text window is open, clicks on the image do nothing.
            let typing = self.text_draft.is_some();
            if response.drag_started() && !typing {
                if let Some(pos) = origin.or_else(|| response.interact_pointer_pos()) {
                    let outcome = self.builder.pointer_down(at(pos));
                    self.handle_outcome(outcome);
                }
            }
            if response.drag_stopped() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let outcome = self.builder.pointer_up(at(pos));
                    self.handle_outcome(outcome);
                }
            } else if response.clicked()
                && !typing
                && (self.builder.is_placing() || tool == Tool::Text)
            {
                if let Some(pos) = response.interact_pointer_pos() {
                    let outcome = self.builder.pointer_down(at(pos));
                    self.handle_outcome(outcome);
                }
            }
            let hint = if tool == Tool::Text && self.text_draft.is_none() {
                Some(tr!("status-text-hint"))
            } else if self.builder.is_placing() {
                Some(tr!("status-place-hint"))
            } else if tool == Tool::Calibrate
                && !self.builder.is_busy()
                && self.calibration.is_none()
            {
                Some(tr!("status-calibrate-hint"))
            } else {
                None
            };
            if let Some(hint) = hint {
                // A dark box keeps the hint readable over a bright image.
                let galley =
                    painter.layout_no_wrap(hint, egui::FontId::proportional(14.0), Color32::WHITE);
                let pos = view.image_rect.left_top() + egui::vec2(8.0, 8.0);
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        pos - egui::vec2(4.0, 2.0),
                        galley.size() + egui::vec2(8.0, 4.0),
                    ),
                    3.0,
                    Color32::from_black_alpha(190),
                );
                painter.galley(pos, galley, Color32::WHITE);
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }

        paint(
            &painter.with_clip_rect(view.image_rect),
            &view,
            &self.overlay_primitives(),
        );
    }
}

#[derive(Default)]
struct Toolbar {
    device: Option<usize>,
    format: Option<Option<usize>>,
    refresh: bool,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.sync_zoom(&ctx);
        self.handle_shortcuts(&ctx);
        self.poll_stream(&ctx);
        self.poll_photo(&ctx);
        self.poll_recorder(&ctx);
        if !self.running() && self.last_refresh.elapsed() >= DEVICE_REFRESH {
            self.refresh_devices(&ctx);
        }
        ctx.request_repaint_after(DEVICE_REFRESH);

        let mut action = Toolbar::default();
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.add_space(4.0);
            action = self.toolbar(ui);
            ui.add_space(2.0);
            self.tools_row(ui);
            ui.add_space(4.0);
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(self.status_text());
                if let Some(warning) = self.mismatch_warning() {
                    ui.separator();
                    ui.colored_label(ui.visuals().warn_fg_color, warning);
                }
                if let Some(text) = self.recording_text() {
                    ui.separator();
                    ui.colored_label(ui.visuals().error_fg_color, text);
                }
                if let Some(notice) = &self.notice {
                    ui.separator();
                    ui.label(notice);
                }
            });
        });
        if self.recent_open {
            egui::Panel::right("recent")
                .resizable(false)
                .show(ui, |ui| self.recent_panel(ui));
        }
        egui::CentralPanel::default().show(ui, |ui| self.canvas(ui));

        self.calibration_window(&ctx);
        self.profiles_window(&ctx);
        self.text_window(&ctx);
        self.settings_window(&ctx);
        self.controls_window(&ctx);
        self.help_window(&ctx);
        self.about_window(&ctx);
        if self.controls_dirty && !ctx.input(|i| i.pointer.any_down()) {
            self.save_controls();
        }

        if let Some(i) = action.device {
            self.device = Some(i);
            self.select_device(&ctx);
        } else if let Some(format) = action.format {
            self.format = format;
            self.reset_view();
            self.error = None;
            self.start_stream(&ctx);
        } else if action.refresh {
            self.refresh_devices(&ctx);
        }
    }
}

/// Mirror a decoded frame and apply the image mode, in place.
fn transform_frame(frame: &mut Frame, horizontal: bool, vertical: bool, mode: ImageMode) {
    let rgba = std::mem::take(&mut frame.rgba);
    if let Some(mut image) = image::RgbaImage::from_raw(frame.width, frame.height, rgba) {
        scopeview_imaging::flip(&mut image, horizontal, vertical);
        scopeview_imaging::apply_mode(&mut image, mode);
        frame.rgba = image.into_raw();
    }
}

/// The language chosen in the settings, or the operating system's.
pub(super) fn language_of(settings: &Settings) -> Language {
    settings
        .language
        .as_deref()
        .and_then(Language::from_code)
        .unwrap_or_else(Language::system)
}

fn mode_label(mode: ImageMode) -> String {
    match mode {
        ImageMode::Normal => tr!("mode-normal"),
        ImageMode::Gray => tr!("mode-gray"),
        ImageMode::Binary => tr!("mode-binary"),
        ImageMode::Invert => tr!("mode-invert"),
        ImageMode::Emboss => tr!("mode-emboss"),
    }
}

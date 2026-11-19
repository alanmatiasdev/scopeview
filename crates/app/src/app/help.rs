//! Keyboard shortcuts, the help window and the about window.

use eframe::egui;
use scopeview_i18n::tr;

use super::App;
use crate::shortcuts::{shortcut_for, Action, TABLE};

/// Where the source code lives, shown in the about window.
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
/// The developer's name (the package author) and profile.
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const AUTHOR_PROFILE: &str = "https://github.com/alanmatiasdev";

impl App {
    /// React to this frame's key presses. Ignored while a text field has the keyboard, so typing
    /// a name never switches tools.
    pub(super) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        for entry in TABLE {
            if ctx.input_mut(|i| i.consume_shortcut(&entry.shortcut)) {
                self.run_action(entry.action);
            }
        }
    }

    fn run_action(&mut self, action: Action) {
        match action {
            Action::Tool(tool) => self.choose_tool(tool),
            Action::Photo => {
                if self.last_frame.is_some() {
                    self.take_photo();
                }
            }
            Action::ToggleRecording => {
                if self.is_recording() {
                    self.stop_recording();
                } else {
                    self.start_recording();
                }
            }
            Action::PauseRecording => self.toggle_pause(),
            Action::DeleteLast => self.delete_last(),
            Action::DeleteAll => self.delete_all(),
            Action::FlipHorizontal => self.flip(true),
            Action::FlipVertical => self.flip(false),
            Action::Crosshair => self.crosshair = !self.crosshair,
            Action::Settings => self.settings_open = !self.settings_open,
            Action::Help => self.help_open = !self.help_open,
        }
    }

    pub(super) fn help_window(&mut self, ctx: &egui::Context) {
        if !self.help_open {
            return;
        }
        let mut open = true;
        egui::Window::new(tr!("help-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                for (heading, tools) in [
                    ("help-section-tools", true),
                    ("help-section-actions", false),
                ] {
                    ui.strong(tr!(heading));
                    egui::Grid::new(heading)
                        .num_columns(2)
                        .spacing([24.0, 4.0])
                        .show(ui, |ui| {
                            for entry in TABLE.iter().filter(|e| e.is_tool() == tools) {
                                ui.label(tr!(entry.label));
                                ui.monospace(ctx.format_shortcut(&entry.shortcut));
                                ui.end_row();
                            }
                        });
                    ui.add_space(6.0);
                }
                ui.weak(tr!("help-other"));
            });
        self.help_open = open;
    }

    pub(super) fn about_window(&mut self, ctx: &egui::Context) {
        if !self.about_open {
            return;
        }
        let mut open = true;
        egui::Window::new(tr!("about-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("ScopeView");
                ui.label(tr!("about-version", version = env!("CARGO_PKG_VERSION")));
                ui.add_space(6.0);
                ui.label(tr!("about-description"));
                ui.label(tr!("about-license"));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(tr!("about-source"));
                    ui.hyperlink_to(REPOSITORY, REPOSITORY);
                });
                ui.horizontal(|ui| {
                    // The developed-by sentence ends with the name, which is the link.
                    let sentence = tr!("about-developed-by", name = AUTHOR);
                    let (before, _) = sentence.rsplit_once(AUTHOR).unwrap_or((&sentence, ""));
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label(before);
                    ui.hyperlink_to(AUTHOR, AUTHOR_PROFILE);
                });
            });
        self.about_open = open;
    }

    /// Hover text for a button: the shortcut that does the same thing.
    pub(super) fn with_shortcut(
        ctx: &egui::Context,
        response: egui::Response,
        action: Action,
    ) -> egui::Response {
        match shortcut_for(action) {
            Some(shortcut) => response.on_hover_text(ctx.format_shortcut(&shortcut)),
            None => response,
        }
    }
}

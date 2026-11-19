//! Keyboard shortcuts as one table, used both to react to keys and to list them in the help window.
//!
//! The key choices are our own. Tools use a single letter; actions that change things use the
//! platform command key (Ctrl, or Cmd on macOS). Esc and Delete keep their drawing-time meaning
//! and are handled where the canvas is.

use eframe::egui::{Key, KeyboardShortcut, Modifiers};

use crate::tools::Tool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Tool(Tool),
    Photo,
    ToggleRecording,
    PauseRecording,
    DeleteLast,
    DeleteAll,
    FlipHorizontal,
    FlipVertical,
    Crosshair,
    Settings,
    Help,
}

/// One row of the shortcut table.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub action: Action,
    pub shortcut: KeyboardShortcut,
    /// Catalog key of the action's name in the UI.
    pub label: &'static str,
}

impl Entry {
    /// Whether the entry belongs in the "Tools" section of the help window.
    pub fn is_tool(&self) -> bool {
        matches!(self.action, Action::Tool(_))
    }
}

const fn plain(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::NONE, key)
}

const fn command(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::COMMAND, key)
}

const fn command_shift(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), key)
}

const fn entry(action: Action, shortcut: KeyboardShortcut, label: &'static str) -> Entry {
    Entry {
        action,
        shortcut,
        label,
    }
}

pub const TABLE: [Entry; 21] = [
    entry(Action::Tool(Tool::None), plain(Key::N), "tool-none"),
    entry(Action::Tool(Tool::Select), plain(Key::V), "tool-select"),
    entry(Action::Tool(Tool::Line), plain(Key::L), "tool-line"),
    entry(Action::Tool(Tool::Rect), plain(Key::R), "tool-rect"),
    entry(Action::Tool(Tool::Circle), plain(Key::C), "tool-circle"),
    entry(
        Action::Tool(Tool::Concentric),
        plain(Key::O),
        "tool-concentric",
    ),
    entry(Action::Tool(Tool::Parallel), plain(Key::P), "tool-parallel"),
    entry(Action::Tool(Tool::Angle), plain(Key::A), "tool-angle"),
    entry(Action::Tool(Tool::Text), plain(Key::T), "tool-text"),
    entry(Action::Tool(Tool::Pencil), plain(Key::D), "tool-pencil"),
    entry(
        Action::Tool(Tool::Calibrate),
        plain(Key::K),
        "tool-calibrate",
    ),
    entry(Action::Photo, plain(Key::Space), "action-photo"),
    entry(Action::ToggleRecording, command(Key::R), "action-record"),
    entry(
        Action::PauseRecording,
        command(Key::P),
        "action-record-pause",
    ),
    entry(Action::DeleteLast, command(Key::Z), "action-delete-last"),
    entry(
        Action::DeleteAll,
        command_shift(Key::Delete),
        "action-delete-all",
    ),
    entry(Action::FlipHorizontal, command(Key::H), "action-flip-h"),
    entry(Action::FlipVertical, command(Key::J), "action-flip-v"),
    entry(Action::Crosshair, plain(Key::X), "action-crosshair"),
    entry(Action::Settings, command(Key::Comma), "action-settings"),
    entry(Action::Help, plain(Key::F1), "action-help"),
];

/// The shortcut for an action, if it has one.
pub fn shortcut_for(action: Action) -> Option<KeyboardShortcut> {
    TABLE
        .iter()
        .find(|e| e.action == action)
        .map(|e| e.shortcut)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: &KeyboardShortcut, b: &KeyboardShortcut) -> bool {
        a.logical_key == b.logical_key && a.modifiers == b.modifiers
    }

    #[test]
    fn no_two_entries_share_a_shortcut() {
        for (i, a) in TABLE.iter().enumerate() {
            for b in &TABLE[i + 1..] {
                assert!(
                    !same(&a.shortcut, &b.shortcut),
                    "{:?} and {:?} share a shortcut",
                    a.action,
                    b.action
                );
            }
        }
    }

    #[test]
    fn every_label_exists_in_the_catalog() {
        for e in TABLE {
            assert!(
                scopeview_i18n::has_key(e.label),
                "missing key {} for {:?}",
                e.label,
                e.action
            );
        }
    }

    #[test]
    fn no_clash_with_the_interface_zoom_keys() {
        for e in TABLE {
            assert!(
                !matches!(
                    e.shortcut.logical_key,
                    Key::Plus | Key::Equals | Key::Minus | Key::Num0
                ),
                "{:?} uses a zoom key",
                e.action
            );
        }
    }

    #[test]
    fn every_tool_has_a_shortcut() {
        for tool in [
            Tool::None,
            Tool::Select,
            Tool::Line,
            Tool::Rect,
            Tool::Circle,
            Tool::Concentric,
            Tool::Parallel,
            Tool::Angle,
            Tool::Text,
            Tool::Pencil,
            Tool::Calibrate,
        ] {
            assert!(shortcut_for(Action::Tool(tool)).is_some(), "{tool:?}");
        }
    }

    #[test]
    fn every_action_has_a_shortcut() {
        for action in [
            Action::Photo,
            Action::ToggleRecording,
            Action::PauseRecording,
            Action::DeleteLast,
            Action::DeleteAll,
            Action::FlipHorizontal,
            Action::FlipVertical,
            Action::Crosshair,
            Action::Settings,
            Action::Help,
        ] {
            assert!(shortcut_for(action).is_some(), "{action:?}");
        }
    }
}

# ADR-0001: egui/eframe for the GUI

- Status: accepted
- Date: 2026-11-19

## Context

ScopeView shows live camera video at up to 30 fps and draws measurement overlays on top,
and must ship as one binary on Linux, Windows and macOS.

## Options considered

- **egui/eframe**: pure Rust, single binary, frames upload straight to a GPU texture,
  immediate-mode painter suits interactive overlays. Non-native look.
- **Tauri 2 + React**: frames must cross into a webview at HD rates; each OS has a
  different webview.
- **Slint**: declarative and store-friendly, but its license adds attribution/GPL constraints.
- **iced**: pure Rust, smaller ecosystem for custom canvas work.

## Decision

Use egui/eframe.

## Consequences

The look is not native. Accessibility and IME support follow egui's. The UI crate stays
thin and depends on `core`, so a different front end remains possible.

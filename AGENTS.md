# AGENTS.md

This file guides coding agents (Claude Code, Codex, Cursor, Gemini CLI, ...) and human
contributors working in this repository. `CLAUDE.md` only imports it, so put new project rules here.

## What this is

ScopeView: a viewer and measurement tool for USB microscopes (UVC cameras) in Rust, for
Linux, Windows and macOS, shipped as a single binary. Feature list in `README.md`.
License: GPL-3.0-or-later.

Everything is in English: code, comments, docs, specs, ADRs, commits.
Commits follow Conventional Commits with crate scopes (`feat(core):`, `fix(camera):`, `docs:`).
Never commit unless asked for that specific commit.

## Commands

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p scopeview --release                       # run the app
cargo test -p scopeview-camera --test hardware -- --ignored --test-threads=1   # needs a camera
```

Building on Linux needs libclang (for `v4l2-sys`). MJPEG decoding must stay pure Rust: do not
enable nokhwa's `decoding` feature (it pulls in mozjpeg).

The Rust toolchain is pinned through asdf (`.tool-versions`: `rust stable`).

## Architecture

Cargo workspace in `crates/`. Dependency direction: `core` <- `imaging`/`camera` <- `record` <- `app` (`record` uses `imaging` to burn in overlays).

- `core`: geometry, units, calibration profiles, measurement shapes, overlay primitives, settings value. Pure (no file access) and unit-tested.
- `camera`: device enumeration, formats, controls, frame threads with reconnect (nokhwa behind our own API). One thread pulls raw frames, another decodes the newest one, so latency never builds up.
- `imaging`: overlay rasterization, photo saving, flip and the image modes (gray/binary/invert/emboss).
- `record`: MJPEG-in-AVI writer and a recorder thread (pacing to a constant frame rate, pause, time limit, file split).
- `i18n`: Fluent bundle in `locales/`; UI strings go through `tr!("key")`, never hard-coded.
- `app` (package `scopeview`, the binary): eframe/egui UI. Pure rules live in `selection.rs` and `view.rs`; config files in `store.rs`.

Decisions are recorded in `docs/adr/`, behavior in `docs/specs/`.

## Feature workflow

Each feature lands as a vertical slice: write or update the spec in `docs/specs/`, write tests,
implement, run the commands above. Measurement math lives in `core` and is unit-tested; UI code
stays thin. Every user-visible string needs an entry in all six `crates/i18n/locales/*/main.ftl`
files (a test checks that they stay in sync).
